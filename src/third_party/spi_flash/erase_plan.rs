use alloc::vec::Vec;
use core::time::Duration;

/// Erase plan of (opcode, size, base address, typical duration) to erase a range of memory.
#[derive(Clone, Debug)]
pub(crate) struct ErasePlan(pub Vec<(u8, usize, u32, Option<Duration>)>);

impl ErasePlan {
    pub fn new(insts: &[(usize, u8, Option<Duration>)], start: usize, length: usize) -> Self {
        let mut plan = Vec::new();
        // Sort instructions by smallest area of effect first.
        let mut insts = insts.to_vec();
        insts.sort();
        // We compute the number of useful bytes erased for each operation,
        // then from those with the same maximum number of useful bytes erased,
        // we select the smallest operation, and repeat until all bytes are erased.
        let end = start + length;
        let mut pos = start;
        while pos < end {
            // Current candidate, (bytes, size, opcode, base).
            let mut candidate = (0, usize::MAX, 0, 0, None);
            for (erase_size, opcode, duration) in insts.iter() {
                let erase_base = pos - (pos % erase_size);
                let erase_end = erase_base + erase_size - 1;
                let mut bytes = erase_size - (pos - erase_base);
                if erase_end > end {
                    bytes -= erase_end - end + 1;
                }
                if bytes > candidate.0 || (bytes == candidate.0 && *erase_size < candidate.1) {
                    candidate = (bytes, *erase_size, *opcode, erase_base, *duration);
                }
            }
            pos += candidate.0;
            plan.push((candidate.2, candidate.1, candidate.3 as u32, candidate.4));
        }
        ErasePlan(plan)
    }
}
