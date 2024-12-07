// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use critical_section::RawRestoreState;
use embedded_alloc::LlffHeap as Heap;

#[global_allocator]
pub static HEAP: Heap = Heap::empty();

struct CriticalSection;
critical_section::set_impl!(CriticalSection);

unsafe impl critical_section::Impl for CriticalSection {
    unsafe fn acquire() -> RawRestoreState {
        // this is a no-op. we're in a single-threaded, non-preemptive context
    }

    unsafe fn release(_token: RawRestoreState) {
        // this is a no-op. we're in a single-threaded, non-preemptive context
    }
}

pub fn init() {
    extern "C" {
        static _end: u8;
    }
    let heap_pos: usize = unsafe { (&_end) as *const u8 as usize };
    let heap_size: usize = crate::memory::GUEST_MAX_MEM - heap_pos;

    unsafe { HEAP.init(heap_pos, heap_size) }
}

/// A no-alloc writer to print to stdout on host machine for debugging purposes.
pub struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        print(s);
        Ok(())
    }
}

fn print(s: &str) {
    let str_as_bytes = s.as_bytes();
    raw_print_str_from_bytes(str_as_bytes.as_ptr(), str_as_bytes.len());
}

fn raw_print_str_from_bytes(msg_ptr: *const u8, len: usize) {
    crate::custom_insn_i!(0x0b, 0b011, msg_ptr, len, 1);
}
