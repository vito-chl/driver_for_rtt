#![allow(unused_variables)]
#![allow(dead_code)]
// Copyright 2020, 2021 Adam Greig
// Licensed under the Apache-2.0 and MIT licenses.

//! spi-flash
//!
//! This crate provides an interface for common SPI flash memories,
//! including discovering ID and parameters, reading, and writing.
use alloc::vec::Vec;
use core::convert::TryInto;
use core::time::Duration;

pub mod erase_plan;
pub mod id;
pub mod sfdp;
pub mod sreg;

pub use id::FlashID;
pub use sfdp::{FlashParams, SFDPAddressBytes, SFDPEraseInst, SFDPStatus1Volatility, SFDPTiming};
pub use sreg::{StatusRegister1, StatusRegister2, StatusRegister3};

use crate::alloc::boxed::Box;
use core::any::Any;
use erase_plan::ErasePlan;
use sfdp::SFDPHeader;

#[derive(Debug)]
pub enum Error {
    ReadBackError { address: u32, wrote: u8, read: u8 },
    InvalidManufacturer,
    InvalidSFDPHeader,
    InvalidSFDPParams,
    InvalidAddress { address: u32 },
    NoResetInstruction,
    NoEraseInstruction,
    Access,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Trait for objects which provide access to SPI flash.
///
/// Providers only need to implement `exchange()`, which asserts CS, writes all the bytes
/// in `data`, then returns all the received bytes. If it provides a performance optimisation,
/// providers may also implement `write()`, which does not require the received data.
///
/// `From<FlashAccess::Error>` must be implemented for `spi_flash::Error`; for example in your
/// implementation code, add:
///
pub trait FlashAccess {
    type Error;

    fn access_init(&self, _param: Box<dyn Any>) -> core::result::Result<(), Self::Error> {
        unimplemented!();
    }

    fn access_uninit(&self) -> core::result::Result<(), Self::Error> {
        unimplemented!();
    }

    /// Assert CS, write all bytes in `data` to the SPI bus, then de-assert CS.
    fn write(&self, data: &[u8]) -> core::result::Result<(), Self::Error> {
        // Default implementation uses `exchange()` and ignores the result data.
        self.exchange(data)?;
        Ok(())
    }

    /// Assert CS, write all bytes in `data` while capturing received data, then de-assert CS.
    ///
    /// Returns the received data.
    fn exchange(&self, data: &[u8]) -> core::result::Result<Vec<u8>, Self::Error>;

    /// Wait for at least `duration`.
    ///
    /// This delay is advisory and reduces polling traffic based on known
    /// typical flash instruction times, so may be left unimplemented.
    ///
    /// The default implementation uses std::thread::delay on std,
    /// and is a no-op on no_std.
    fn delay(&self, duration: Duration);
}

/// SPI Flash.
///
/// This struct provides methods for interacting with common SPI flashes.
pub struct Flash<A: FlashAccess> {
    pub access: A,

    /// Once read, ID details are cached.
    id: Option<FlashID>,

    /// Once read, SFDP parameters are cached.
    params: Option<FlashParams>,

    /// Number of address bytes to use when reading and writing.
    /// This is set to 3 by default for compatibility, but may
    /// be set to 2 for legacy memories or 4 for high-density memories.
    address_bytes: u8,

    /// Total data memory capacity in bytes, up to 4GB.
    pub capacity: Option<usize>,

    /// Page size in bytes, used for programming operations.
    pub page_size: Option<usize>,

    /// Sector size in bytes, used for the smallest erase operations.
    pub erase_size: Option<usize>,

    /// EraseSector instruction opcode.
    /// This is set to 0x20 by default but may be overridden.
    erase_opcode: u8,
}

impl<'a, A: FlashAccess> Flash<A>
where
    Error: From<<A as FlashAccess>::Error>,
{
    /// Create a new Flash instance using the given FlashAccess provider.
    pub fn new(access: A) -> Self {
        Flash {
            access,
            id: None,
            params: None,
            address_bytes: 3,
            capacity: None,
            page_size: None,
            erase_size: None,
            erase_opcode: 0x20,
        }
    }

    /// Get the number of address bytes which will be used in read and write commands.
    pub fn address_bytes(&self) -> u8 {
        self.address_bytes
    }

    /// Set the number of address bytes to use with read and write commands.
    /// By default this is set to 3, and can also be autodiscovered using SFDP.
    ///
    /// Panics if `n` is less than 1 or greater than 4.
    pub fn set_address_bytes(&mut self, n: u8) {
        assert!(n >= 1, "set_address_bytes: n must be at least 1");
        assert!(n <= 4, "set_address_bytes: n must not exceed 4");
        self.address_bytes = n;
    }

    /// Get the total memory capacity in bytes, if known.
    pub fn capacity(&self) -> Option<usize> {
        self.capacity
    }

    /// Set the total memory capacity in bytes.
    ///
    /// If set, or discovered using SFDP, reads and writes are prevented
    /// from going beyond the memory capacity.
    pub fn set_capacity(&mut self, n: usize) {
        self.capacity = Some(n);
    }

    /// Get the page program size in bytes.
    pub fn page_size(&self) -> Option<usize> {
        self.page_size
    }

    /// Set the page program size in bytes.
    ///
    /// This must be known before page program operations can be performed.
    pub fn set_page_size(&mut self, n: usize) {
        self.page_size = Some(n);
    }

    /// Get the sector erase size in bytes, if known.
    pub fn erase_size(&self) -> Option<usize> {
        self.erase_size
    }

    /// Set the sector erase size in bytes.
    ///
    /// This must be known before sector erase operations can be performed.
    pub fn set_erase_size(&mut self, n: usize) {
        self.erase_size = Some(n);
    }

    /// Get the opcode used for the Erase Sector instruction.
    pub fn erase_opcode(&self) -> u8 {
        self.erase_opcode
    }

    /// Set the opcode used for the Erase Sector instruction.
    ///
    /// This is 0x20 by default.
    pub fn set_erase_opcode(&mut self, opcode: u8) {
        self.erase_opcode = opcode;
    }

    /// Get the flash ID, if it has already been read.
    ///
    /// Call `read_id()` to read the ID from the flash.
    pub fn get_id(&self) -> Option<FlashID> {
        self.id
    }

    /// Get the flash parameters, if they have already been read.
    ///
    /// Call `read_params()` to read the params from the flash.
    pub fn get_params(&self) -> Option<FlashParams> {
        self.params
    }

    /// Read the device's manufacturer ID and device IDs.
    ///
    /// This method additionally brings the flash out of powerdown and resets it.
    ///
    /// `self.id` is updated with the new ID; use `get_id()` to
    /// retrieve it without re-reading the ID from the flash.
    pub fn read_id(&mut self) -> Result<FlashID> {
        let legacy_id = self.release_power_down()?;
        self.reset()?;

        let (bank_long, mfn_id_long, device_id_long) = self.read_jedec_id()?;
        let (bank_short, mfn_id_short, mut device_id_short) = self.read_device_id()?;
        let unique_id = self.read_unique_id()?;

        // The device may implement any or none of the three identification
        // instructions; attempt to obtain a valid manufacturer ID and device ID.
        // If there is no device present or a communication error, we'll probably
        // receive all-0s or all-1s for all the data.
        let manufacturer_bank;
        let manufacturer_id;
        if mfn_id_long != 0x00 && mfn_id_long != 0xFF {
            manufacturer_bank = bank_long;
            manufacturer_id = mfn_id_long;
        } else if mfn_id_short != 0x00 && mfn_id_short != 0xFF {
            manufacturer_bank = bank_short;
            manufacturer_id = mfn_id_short;
        } else {
            if legacy_id == 0x00 || legacy_id == 0xFF {
                return Err(Error::InvalidManufacturer);
            } else {
                device_id_short = legacy_id;
                manufacturer_bank = 0;
                manufacturer_id = 0;
            }
        }

        let id = FlashID {
            manufacturer_bank,
            manufacturer_id,
            device_id_long,
            device_id_short,
            unique_id,
        };
        self.id = Some(id);
        Ok(id)
    }

    /// Read SFDP JEDEC Basic Flash Parameter table from flash.
    ///
    /// Access errors are returned as usual, but if SFDP is not supported
    /// (no SFDP signature is detected in the first SFDP DWORD) then
    /// `Ok(None)` is returned instead.
    ///
    /// Depending on the version of SFDP supported, some fields may
    /// not be available.
    ///
    /// Once read, the parameters are available using `get_params()`,
    /// and the parameter values are automatically used for the
    /// configuration of address bytes, capacity, page size, sector size,
    /// and erase sector opcode. Additionally, larger erase commands
    /// described by the SFDP parameters will be used when appropriate.
    pub fn read_params(&mut self) -> Result<Option<FlashParams>> {
        // Read just SFDP header to get NPH first.
        let data = self.read_sfdp(0, 8)?;
        let nph = data[6] as usize + 1;

        // Re-read overall SFDP header including parameter headers.
        // Handle errors parsing the header by returning Ok(None),
        // since not all flash devices support SFDP.
        // After this parse is successful, however, subsequent errors
        // are returned as errors.
        let data = self.read_sfdp(0, 8 + nph * 8)?;
        let header = match SFDPHeader::from_bytes(&data) {
            Ok(header) => header,
            Err(_) => return Ok(None),
        };

        // Check the first parameter header is the JEDEC basic flash parameters,
        // as required by JESD216.
        let params = header.params[0];
        if params.parameter_id != 0xFF00 || params.major != 0x01 {
            return Err(Error::InvalidSFDPHeader);
        }

        // Read SFDP table data and parse into a FlashParams struct.
        let data = self.read_sfdp(params.ptp, params.plen * 4)?;
        let params = FlashParams::from_bytes(params.major, params.minor, &data)?;
        self.params = Some(params);

        // Use params to update settings where posssible.
        self.address_bytes = match params.address_bytes {
            SFDPAddressBytes::Three => 3,
            SFDPAddressBytes::ThreeOrFour => 3,
            SFDPAddressBytes::Four => 4,
            _ => 3,
        };
        self.capacity = Some(params.capacity_bytes());
        if let Some(page_size) = params.page_size {
            self.page_size = Some(page_size as usize);
        }
        if let Some((size, opcode)) = params.sector_erase() {
            self.erase_size = Some(size);
            self.erase_opcode = opcode;
        } else if params.legacy_4kb_erase_supported && params.legacy_4kb_erase_inst != 0xFF {
            self.erase_size = Some(4096);
            self.erase_opcode = params.legacy_4kb_erase_inst;
        }
        Ok(Some(params))
    }

    /// Read `length` bytes of data from the attached flash, starting at `address`.
    ///
    /// This method uses the FastRead instruction; if it is not supported
    /// try using `legacy_read()` instead.
    pub fn read(&mut self, address: u32, length: usize) -> Result<Vec<u8>> {
        self.check_address_length(address, length)?;
        let mut param = self.make_address(address);
        // Dummy byte after address.
        param.push(0);
        self.exchange(Command::FastRead, &param, length)
    }

    /// Read `length` bytes of data from the attached flash, starting at `address`.
    ///
    /// This method uses the legacy ReadData instruction, which often has a low
    /// maximum clock speed compared to other operations, but is more widely supported
    /// and may be faster for very short reads as it does not require a dummy byte.
    pub fn legacy_read(&mut self, address: u32, length: usize) -> Result<Vec<u8>> {
        self.check_address_length(address, length)?;
        let param = self.make_address(address);
        self.exchange(Command::ReadData, &param, length)
    }

    /// Read `length` bytes of data from the attached flash, starting at `address`.
    ///
    /// This method is similar to the `read()` method, except it calls the provided
    /// callback function at regular intervals with the number of bytes read so far.
    ///
    /// While `read()` performs a single long SPI exchange, this method performs
    /// up to 128 separate SPI exchanges to allow progress to be reported.
    pub fn read_cb<F: Fn(usize)>(&mut self, address: u32, length: usize, cb: F) -> Result<Vec<u8>> {
        self.check_address_length(address, length)?;
        let chunk_size = usize::max(1024, length / 128);
        let start = address as usize;
        let end = start + length;
        let mut data = Vec::new();
        cb(0);
        for addr in (start..end).step_by(chunk_size) {
            let size = usize::min(chunk_size, end - addr);
            let mut param = self.make_address(addr as u32);
            param.push(0);
            data.append(&mut self.exchange(Command::FastRead, &param, size)?);
            cb(data.len());
        }
        cb(data.len());
        Ok(data)
    }

    /// Erase entire flash chip.
    ///
    /// This method uses the ChipErase instruction, so no progress information
    /// is available, but the typical and maximum times taken may be available
    /// from the SFDP parameters.
    ///
    /// Returns only after erase operation is complete.
    pub fn erase(&mut self) -> Result<()> {
        self.write_enable()?;
        self.command(Command::ChipErase)?;
        self.wait_while_busy()?;
        Ok(())
    }

    /// Program the attached flash with `data` starting at `address`.
    ///
    /// Sectors and blocks are erased as required for the new data,
    /// and existing data outside the new data to write is written
    /// back if it has to be erased.
    ///
    /// If `verify` is true, the programmed data is read back, and
    /// a ReadbackError will be returned if it did not match what was written.
    ///
    /// When available, SFDP parameters are used to generate an efficient
    /// sequence of erase instructions. If unavailable, the single erase
    /// instruction in `erase_opcode` is used, and its size of effect
    /// must be given in `erase_size`. If these are not set, a
    /// `NoEraseInstruction` is returned.
    ///
    /// The programming page size is set by `page_size`, which is
    /// automatically set when SFDP parameters are read.
    pub fn program(&mut self, address: u32, data: &[u8], verify: bool) -> Result<()> {
        self.check_address_length(address, data.len())?;

        // Work out a good erasure plan.
        let erase_plan = self.make_erase_plan(address, data.len())?;

        // Read data which will be inadvertently erased so we can restore it.
        let full_data = self.make_restore_data(address, data, &erase_plan)?;

        // Execute erasure plan.
        self.run_erase_plan(&erase_plan, |_| {})?;

        // Write new data.
        let start_addr = erase_plan.0[0].2;
        self.program_data(start_addr, &full_data)?;

        // Optionally do a readback to verify all written data.
        if verify {
            let programmed = self.read(start_addr, full_data.len())?;
            self.verify_read_back(start_addr, &full_data, &programmed)?;
        }

        Ok(())
    }

    /// Reset the attached flash.
    ///
    /// The instruction sequence EnableReset 0x66 followed by Reset 0x99
    /// is sent by default, but if the SFDP parameters indicate that only
    /// the 0xF0 instruction is supported for reset, that is sent instead.
    pub fn reset(&mut self) -> Result<()> {
        let mut do_f0 = false;
        let mut do_66_99 = true;

        if let Some(params) = self.params {
            if let Some(op_66_99) = params.reset_inst_66_99 {
                do_66_99 = op_66_99;
            }
            if let Some(op_f0) = params.reset_inst_f0 {
                do_f0 = op_f0;
            }
        }

        if do_66_99 {
            self.command(Command::EnableReset)?;
            self.command(Command::Reset)
        } else if do_f0 {
            self.command(0xF0)
        } else {
            Err(Error::NoResetInstruction)
        }
    }

    /// Check if any block protect bits are set in status register 1.
    pub fn is_protected(&mut self) -> Result<bool> {
        let status1 = self.read_status1()?;
        let (bp0, bp1, bp2) = status1.get_block_protect();
        Ok(bp0 || bp1 || bp2)
    }

    /// Set block protection bits.
    ///
    /// This sets the block protect bits in status register 1,
    /// using the non-volatile commands if supported. If available,
    /// the SFDP parameters are used to determine the correct
    /// non-volatile instruction.
    pub fn protect(&mut self, bp0: bool, bp1: bool, bp2: bool) -> Result<()> {
        let mut status1 = self.read_status1()?;
        status1.set_block_protect(bp0, bp1, bp2);
        self.write_status1(status1)?;
        self.wait_while_busy()?;
        Ok(())
    }

    /// Clear any protection bits that are set.
    ///
    /// This checks and clears the block protect bits in status register 1,
    /// using the non-volatile commands if supported. If available, the SFDP
    /// parameters are used to determine the correct non-volatile instruction.
    pub fn unprotect(&mut self) -> Result<()> {
        let mut status1 = self.read_status1()?;
        let (bp0, bp1, bp2) = status1.get_block_protect();
        if bp0 || bp1 || bp2 {
            status1.set_block_protect(false, false, false);
            self.write_status1(status1)?;
            self.wait_while_busy()?;
        }
        Ok(())
    }

    /// Clear the write-protect-selection bit in status register 3, if set.
    ///
    /// This status bit configures the fine-granularity write protection
    /// which is a vendor-specific extension that is disabled by default.
    ///
    /// Unfortunately it is not possible to determine automatically if a
    /// flash chip supports the WPS feature or even has a status register 3,
    /// so this command is not called automatically.
    pub fn unprotect_wps(&mut self) -> Result<()> {
        let mut status3 = self.read_status3()?;
        if status3.get_wps() {
            status3.set_wps(false);
            self.write_status3(status3)?;
            self.wait_while_busy()?;
        }
        Ok(())
    }

    /// Power down the flash.
    pub fn power_down(&mut self) -> Result<()> {
        self.command(Command::PowerDown)
    }

    /// Power up the flash.
    ///
    /// Returns the legacy device ID.
    pub fn release_power_down(&mut self) -> Result<u8> {
        let data = self.exchange(Command::ReleasePowerDown, &[0, 0, 0], 1)?;
        Ok(data[0])
    }

    /// Program `data` to `address`, automatically split into multiple page program operations.
    ///
    /// Note that this does *not* erase the flash beforehand; use `program()` for a higher-level
    /// erase-program-verify interface.
    pub fn program_data(&mut self, address: u32, data: &[u8]) -> Result<()> {
        self.program_data_cb(address, data, |_| {})
    }

    /// Program `data` to `address`, automatically split into multiple page program operations.
    ///
    /// Note that this does *not* erase the flash beforehand; use `program()` for a higher-level
    /// erase-program-verify interface.
    ///
    /// Calls `cb` with the number of bytes programmed so far after each
    /// page programming operation.
    pub fn program_data_cb<F: Fn(usize)>(
        &mut self,
        address: u32,
        mut data: &[u8],
        cb: F,
    ) -> Result<()> {
        let page_size = match self.page_size {
            Some(page_size) => page_size,
            None => 256,
        };

        let mut total_bytes = 0;
        cb(total_bytes);

        // If the address is not page-aligned, we need to do a
        // smaller-than-page-size initial program.
        let first_write = page_size - ((address as usize) % page_size);
        if first_write != page_size {
            self.page_program(address, &data[..first_write])?;
            total_bytes += first_write;
            data = &data[first_write..];
            cb(total_bytes);
        }

        for page_data in data.chunks(page_size) {
            self.page_program(address + total_bytes as u32, page_data)?;
            total_bytes += page_data.len();
            cb(total_bytes);
        }

        Ok(())
    }

    /// Send the WriteEnable command, setting the WEL in the status register.
    pub fn write_enable(&mut self) -> Result<()> {
        self.command(Command::WriteEnable)
    }

    /// Program up to one page of data.
    ///
    /// This method sets the write-enable latch and then waits for programming to complete,
    /// including sleeping half the typical page program time if known before polling.
    ///
    /// Note that this does *not* erase the flash beforehand;
    /// use `program()` for a higher-level erase-program-verify interface.
    pub fn page_program(&mut self, address: u32, data: &[u8]) -> Result<()> {
        let mut tx = self.make_address(address);
        tx.extend(data);
        self.write_enable()?;
        self.exchange(Command::PageProgram, &tx, 0)?;
        if let Some(params) = self.params {
            if let Some(timing) = params.timing {
                // Only bother sleeping if the expected programming time is greater than 1ms,
                // otherwise we'll likely have waited long enough just due to round-trip delays.
                // We always poll the status register at least once to check write completion.
                if timing.page_prog_time_typ > Duration::from_millis(1) {
                    self.access.delay(timing.page_prog_time_typ / 2);
                }
            }
        }
        self.wait_while_busy()?;
        Ok(())
    }

    /// Reads the JEDEC manufacturer and long (16-bit) device IDs.
    ///
    /// The manufacturer ID may be prefixed with up to 13 of the
    /// continuation code 0x7F; the number of continuation codes
    /// is returned as the bank number.
    ///
    /// Returns (bank, manufacturer ID, device ID).
    pub fn read_jedec_id(&mut self) -> Result<(u8, u8, u16)> {
        // Attempt to read assuming a single-byte manufacturer ID.
        let data = self.exchange(Command::ReadJEDECID, &[], 3)?;
        if data[0] != 0x7F {
            Ok((0, data[0], u16::from_be_bytes([data[1], data[2]])))
        } else {
            // If the first byte is continuation, read 16 bytes, to allow
            // up to 13 continuation bytes, and then parse it to find the IDs.
            let data = self.exchange(Command::ReadJEDECID, &[], 16)?;
            for n in 1..=13 {
                if data[n] != 0x7F {
                    return Ok((
                        n as u8,
                        data[n],
                        u16::from_be_bytes([data[n + 1], data[n + 2]]),
                    ));
                }
            }
            Err(Error::InvalidManufacturer)
        }
    }

    /// Reads the JEDEC manufacturer and short (8-bit) device IDs.
    ///
    /// The manufacturer ID may be prefixed with up to 13 of the
    /// continuation code 0x7F; the number of continuation codes
    /// is returned as the bank number.
    ///
    /// Returns (bank, manufacturer ID, device ID).
    pub fn read_device_id(&mut self) -> Result<(u8, u8, u8)> {
        // Attempt to read assuming a single-byte manufacturer ID.
        let data = self.exchange(Command::ReadDeviceID, &[0, 0, 0], 2)?;
        if data[0] != 0x7F {
            Ok((0, data[0], data[1]))
        } else {
            // If the first byte is continuation, read 15 bytes, to allow
            // up to 13 continuation bytes, and then parse it to find the IDs.
            let data = self.exchange(Command::ReadJEDECID, &[0, 0, 0], 15)?;
            for n in 1..=13 {
                if data[n] != 0x7F {
                    return Ok((n as u8, data[n], data[n + 1]));
                }
            }
            Err(Error::InvalidManufacturer)
        }
    }

    /// Read the device's 64-bit unique ID, if present.
    pub fn read_unique_id(&mut self) -> Result<u64> {
        self.exchange(Command::ReadUniqueID, &[0, 0, 0, 0], 8)
            .map(|data| u64::from_be_bytes(data.try_into().unwrap()))
    }

    /// Read status register 1.
    pub fn read_status1(&mut self) -> Result<StatusRegister1> {
        self.exchange(Command::ReadStatusRegister1, &[], 1)
            .map(|data| StatusRegister1(data[0]))
    }

    /// Read status register 2.
    ///
    /// This status register is less widely supported and SFDP does
    /// not indicate whether or not it is present.
    pub fn read_status2(&mut self) -> Result<StatusRegister2> {
        self.exchange(Command::ReadStatusRegister2, &[], 1)
            .map(|data| StatusRegister2(data[0]))
    }

    /// Read status register 3.
    ///
    /// This status register is less widely supported and SFDP does
    /// not indicate whether or not it is present.
    pub fn read_status3(&mut self) -> Result<StatusRegister3> {
        self.exchange(Command::ReadStatusRegister3, &[], 1)
            .map(|data| StatusRegister3(data[0]))
    }

    /// Write status register 1.
    ///
    /// This method does *not* require you call `write_enable()` first.
    ///
    /// If the SFDP parameters indicate a specific command should be used
    /// to enable writing to status register 1, that is used, otherwise the
    /// default WriteEnable of 0x06 is used.
    fn write_status1(&mut self, status1: StatusRegister1) -> Result<()> {
        let we_opcode = if let Some(params) = self.params {
            match params.status_1_vol {
                Some(SFDPStatus1Volatility::NonVolatile06) => 0x06,
                Some(SFDPStatus1Volatility::Volatile06) => 0x06,
                Some(SFDPStatus1Volatility::Volatile50) => 0x50,
                Some(SFDPStatus1Volatility::NonVolatile06Volatile50) => 0x06,
                Some(SFDPStatus1Volatility::Mixed06) => 0x06,
                _ => {
                    if params.legacy_block_protect_volatile {
                        params.legacy_volatile_write_en_inst
                    } else {
                        Command::WriteEnable.into()
                    }
                }
            }
        } else {
            Command::WriteEnable.into()
        };
        self.command(we_opcode)?;
        let s1 = self.read_status1()?;
        self.write(Command::WriteStatusRegister1, &[status1.0])
    }

    /// Write status register 2.
    pub fn write_status2(&mut self, status2: StatusRegister2) -> Result<()> {
        self.write_enable()?;
        self.write(Command::WriteStatusRegister2, &[status2.0])
    }

    /// Write status register 3.
    pub fn write_status3(&mut self, status3: StatusRegister3) -> Result<()> {
        self.write_enable()?;
        self.write(Command::WriteStatusRegister3, &[status3.0])
    }

    /// Check if the device is currently busy performing an operation.
    ///
    /// If the flash parameters indicate support for the Flag Status Register
    /// instruction (0x70), it is used, otherwise legacy polling of status
    /// register 1 is used.
    pub fn is_busy(&mut self) -> Result<bool> {
        // If we have read parameters and flag status polling is supported, use that.
        // Bit 7 of FSR is 0=busy and 1=ready.
        if let Some(params) = self.params {
            if let Some(busy_poll_flag) = params.busy_poll_flag {
                if busy_poll_flag {
                    let fsr = self.exchange(Command::ReadFlagStatusRegister, &[], 1)?[0];
                    return Ok(fsr & 0b1000_0000 == 0);
                }
            }
        }

        // Otherwise and by default, poll status register 1 instead.
        self.read_status1().map(|status| status.get_busy())
    }

    /// Wait until the device stops being busy.
    ///
    /// This polls using `is_busy()`, which uses the flag status
    /// register if available or otherwise uses status register 1.
    pub fn wait_while_busy(&mut self) -> Result<()> {
        while self.is_busy()? {}
        Ok(())
    }

    /// Read SFDP register data.
    ///
    /// `addr` is always sent as a 24-bit address, regardless of the address_bytes setting.
    pub fn read_sfdp(&mut self, addr: u32, len: usize) -> Result<Vec<u8>> {
        let bytes = addr.to_be_bytes();
        self.exchange(Command::ReadSFDPRegister, &bytes[1..], 1 + len)
            .map(|data| data[1..].to_vec())
    }

    /// Writes `command` and `data` to the flash memory, then returns `nbytes` of response.
    pub fn exchange<C: Into<u8>>(
        &mut self,
        command: C,
        data: &[u8],
        nbytes: usize,
    ) -> Result<Vec<u8>> {
        let mut tx = alloc::vec![command.into()];
        tx.extend(data);
        tx.extend(alloc::vec![0u8; nbytes]);
        let rx = self.access.exchange(&tx)?;
        Ok(rx[1 + data.len()..].to_vec())
    }

    /// Writes `command` and `data` to the flash memory, without reading the response.
    pub fn write<C: Into<u8>>(&mut self, command: C, data: &[u8]) -> Result<()> {
        let mut tx = alloc::vec![command.into()];
        tx.extend(data);
        self.access.write(&tx)?;
        Ok(())
    }

    /// Convenience method for issuing a single command and not caring about the returned data
    pub fn command<C: Into<u8>>(&mut self, command: C) -> Result<()> {
        self.write(command, &[])?;
        Ok(())
    }

    /// Checks if `address` and `length` together are permissible:
    /// * `address` must not exceed the current number of address bytes
    /// * Both `address` and `address+length` must be within the flash memory bounds,
    ///   if the capacity is known.
    /// Returns either Err(Error::InvalidAddress) or Ok(()).
    fn check_address_length(&self, address: u32, length: usize) -> Result<()> {
        let start = address as usize;
        let end = (address as usize) + length - 1;
        let max_addr = 1 << (self.address_bytes * 8);

        if (end & (max_addr - 1)) < start {
            Err(Error::InvalidAddress {
                address: end as u32,
            })
        } else if end > max_addr {
            Err(Error::InvalidAddress {
                address: end as u32,
            })
        } else {
            match self.capacity {
                Some(capacity) if (end >= capacity) => Err(Error::InvalidAddress {
                    address: end as u32,
                }),
                _ => Ok(()),
            }
        }
    }

    /// Generate a 1-, 2-, 3-, or 4-byte address, depending on current `address_bytes` setting.
    ///
    /// Panics if address_bytes is not 1-, 2, 3, or 4.
    fn make_address(&self, addr: u32) -> Vec<u8> {
        let bytes = addr.to_be_bytes();
        bytes[(4 - self.address_bytes as usize)..].to_vec()
    }

    /// Work out what combination of erase operations to run to efficiently
    /// erase the specified memory.
    fn make_erase_plan(&self, address: u32, length: usize) -> Result<ErasePlan> {
        // Erase instructions: (size in bytes, opcode).
        let mut insts = Vec::new();

        // Find available erase instructions.
        if let Some(params) = self.params {
            if params.erase_insts.iter().any(|&inst| inst.is_some()) {
                for inst in params.erase_insts.iter().flatten() {
                    insts.push((inst.size as usize, inst.opcode, inst.time_typ));
                }
            } else if params.legacy_4kb_erase_supported {
                insts.push((4096, params.legacy_4kb_erase_inst, None));
            } else {
            }
        }
        if insts.is_empty() {
            if let Some(erase_size) = self.erase_size {
                insts.push((erase_size, self.erase_opcode, None));
            } else {
                return Err(Error::NoEraseInstruction);
            }
        }
        insts.sort();

        // Create plan given the list of available erase instructions.
        Ok(ErasePlan::new(&insts, address as usize, length))
    }

    /// Read all the bytes before `address` in memory which will be erased by `plan`.
    fn read_erase_preamble(&mut self, address: u32, plan: &ErasePlan) -> Result<Vec<u8>> {
        let base = plan.0[0].2;
        let len = address - base;
        if len > 0 {
            self.read(base, len as usize)
        } else {
            Ok(Vec::new())
        }
    }

    /// Read all the bytes after `address + length` in memory which will be erased by `plan`.
    ///
    /// If all those bytes are 0xFF, returns an empty Vec instead, as they won't be changed
    /// by the erase operation.
    fn read_erase_postamble(
        &mut self,
        address: u32,
        length: usize,
        plan: &ErasePlan,
    ) -> Result<Vec<u8>> {
        let (_, size, base, _) = plan.0.last().unwrap();
        let start = address + (length as u32);
        let len = (*base as usize + *size) - start as usize;
        if len > 0 {
            let data = self.read(start, len)?;
            // If all the postamble is already 0xFF, there's no point reprogramming it.
            if data.iter().all(|x| *x == 0xFF) {
                Ok(Vec::new())
            } else {
                Ok(data)
            }
        } else {
            Ok(Vec::new())
        }
    }

    /// Extend `data` by adding any preamble and postamble required to preserve
    /// existing data after erasing and reprogramming.
    fn make_restore_data(
        &mut self,
        address: u32,
        data: &[u8],
        erase_plan: &ErasePlan,
    ) -> Result<Vec<u8>> {
        let preamble = self.read_erase_preamble(address, &erase_plan)?;
        let postamble = self.read_erase_postamble(address, data.len(), &erase_plan)?;
        let mut full_data = preamble;
        full_data.extend(data);
        full_data.extend(&postamble);
        Ok(full_data)
    }

    /// Execute the sequence of erase operations from `plan`.
    ///
    /// `cb` is called with the number of bytes erased so far.
    fn run_erase_plan<F: Fn(usize)>(&mut self, plan: &ErasePlan, cb: F) -> Result<()> {
        let mut total_erased = 0;
        cb(total_erased);
        for (opcode, size, base, duration) in plan.0.iter() {
            let addr = self.make_address(*base);
            self.write_enable()?;
            self.write(*opcode, &addr)?;
            if let Some(duration) = duration {
                self.access.delay(*duration / 2);
            }
            self.wait_while_busy()?;
            total_erased += size;
            cb(total_erased);
        }
        cb(total_erased);
        Ok(())
    }

    /// Verify programmed data matches new flash contents.
    ///
    /// Returns Err::ReadBackError on mismatch.
    fn verify_read_back(&mut self, address: u32, data: &[u8], new_data: &[u8]) -> Result<()> {
        let mismatch = data
            .iter()
            .zip(new_data)
            .enumerate()
            .find(|(_, (a, b))| a != b);
        match mismatch {
            Some((idx, (a, b))) => {
                let addr = address + idx as u32;
                if self.is_protected()? {}
                Err(Error::ReadBackError {
                    address: addr,
                    wrote: *a,
                    read: *b,
                })
            }
            None => Ok(()),
        }
    }
}

/// Standard SPI flash command opcodes.
///
/// These are taken from the Winbond W25Q16JV datasheet, but most are
/// widely applicable. If SFDP is supported, it is used to discover
/// the relevant erase opcodes and sizes.
///
/// Only single I/O commands are listed.
#[derive(Copy, Clone, Debug, num_enum::IntoPrimitive)]
#[allow(unused)]
#[repr(u8)]
enum Command {
    // Core instruction set.
    // These commands are almost universally available.
    WriteEnable = 0x06,
    WriteDisable = 0x04,
    ReadData = 0x03,
    PageProgram = 0x02,
    ReadStatusRegister1 = 0x05,
    WriteStatusRegister1 = 0x01,

    // Standard instruction set.
    // These commands are typically available.
    ReadJEDECID = 0x9F,
    FastRead = 0x0B,
    PowerDown = 0xB9,
    ReleasePowerDown = 0xAB,
    ReadDeviceID = 0x90,
    ChipErase = 0xC7,

    // Extended instruction set.
    // These commands may be available.
    ReadUniqueID = 0x4B,
    ReadSFDPRegister = 0x5A,
    ReadStatusRegister2 = 0x35,
    ReadStatusRegister3 = 0x15,
    ReadFlagStatusRegister = 0x70,
    WriteStatusRegister2 = 0x31,
    WriteStatusRegister3 = 0x11,
    WriteEnableVolatile = 0x50,
    EnableReset = 0x66,
    Reset = 0x99,
    ProgramSuspend = 0x75,
    ProgramResume = 0x7A,

    // Erase instructions.
    // The size affected by each erase operation can vary.
    // Typical sizes are 4kB for sector erase, 32kB for block erase 1,
    // and 64kB for block erase 2.
    SectorErase = 0x20,
    BlockErase1 = 0x52,
    BlockErase2 = 0xD8,

    // Security/lock related instructions.
    EraseSecurityRegisters = 0x44,
    ProgramSecurityRegisters = 0x42,
    ReadSecurityRegisters = 0x48,
    IndividualBlockLock = 0x36,
    IndividualBlockUnlock = 0x39,
    ReadBlockLock = 0x3D,
    GlobalBlockLock = 0x7E,
    GlobalBlockUnlock = 0x98,
}
