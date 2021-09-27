/// Store the ID read off an SPI flash memory.
///
/// The manufacturer ID and (long, 16-bit) device ID are read using the 0x9F command,
/// and the number of 0x7F continuation code bytes present before the manufacturer ID
/// is stored as `manufacturer_bank`.
///
/// The 64-bit unique ID is read using the 0x4B command.
#[derive(Copy, Clone, Debug)]
pub struct FlashID {
    pub manufacturer_bank: u8,
    pub manufacturer_id: u8,
    pub device_id_long: u16,
    pub device_id_short: u8,
    pub unique_id: u64,
}
