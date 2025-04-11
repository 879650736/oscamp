#![no_std]
#![allow(unused_variables)]

use allocator::{BaseAllocator, ByteAllocator, AllocResult};
use core::ptr::NonNull;
use core::alloc::Layout;

// Memory block metadata structure
#[repr(C)]
struct Block {
    start: usize,
    size: usize,
    is_free: bool,
    next: Option<usize>, // Offset to next block
    prev: Option<usize>,
}

pub struct LabByteAllocator {
    memory_start: usize,
    memory_size: usize,
    first_block: Option<usize>, // Offset to first block
    used_bytes: usize,
}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            memory_start: 0,
            memory_size: 0,
            first_block: None,
            used_bytes: 0,
        }
    }
    
    // Helper function to get block at specific address
    unsafe fn get_block(&self, addr: usize) -> &Block {
        &*(addr as *const Block)
    }
    
    // Helper function to get mutable block at specific address
    unsafe fn get_block_mut(&mut self, addr: usize) -> &mut Block {
        &mut *(addr as *mut Block)
    }
    
    fn find_free_block(&self, size: usize, align: usize) -> Option<usize> {
        let mut current = self.first_block;
        let mut best_fit_block: Option<usize> = None;
        let mut best_fit_waste = usize::MAX;

        while let Some(block_addr) = current {
            let block = unsafe { self.get_block(block_addr) };
            if block.is_free {
                let data_addr = block_addr + core::mem::size_of::<Block>();
                let aligned_addr = (data_addr + align - 1) & !(align - 1);
                let padding = aligned_addr - data_addr;

                if block.size >= size + padding {
                    // Calculate waste (padding + unused space)
                    let waste = padding + (block.size - size - padding);
                    if waste < best_fit_waste {
                        best_fit_waste = waste;
                        best_fit_block = Some(block_addr);
                    }
                }
            }
            current = block.next;
        }
        best_fit_block
    }
    // New method to find the largest contiguous free block
    fn largest_free_block(&self) -> usize {
        let mut largest = 0;
        let mut current = self.first_block;
        while let Some(block_addr) = current {
            let block = unsafe { self.get_block(block_addr) };
            if block.is_free && block.size > largest {
                largest = block.size;
            }
            current = block.next;
        }
        largest
    }
}

impl BaseAllocator for LabByteAllocator {
    fn init(&mut self, start: usize, size: usize) {
        self.memory_start = start;
        self.memory_size = size;
        self.used_bytes = 0;
        
        // Create initial free block
        unsafe {
            let block = self.get_block_mut(start);
            *block = Block {
                start,
                size: size - core::mem::size_of::<Block>(),
                is_free: true,
                next: None,
                prev: None,
            };
        }
        
        self.first_block = Some(start);
    }
    
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // Create new block
        unsafe {
            let new_block = self.get_block_mut(start);
            *new_block = Block {
                start,
                size: size - core::mem::size_of::<Block>(),
                is_free: true,
                next: None,
                prev: None,
            };
        }
        
        // Find the last block
        let mut current = self.first_block;
        let mut last_block_addr = 0;
        
        while let Some(block_addr) = current {
            let block = unsafe { self.get_block(block_addr) };
            if block.next.is_none() {
                last_block_addr = block_addr;
                break;
            }
            current = block.next;
        }
        
        
        // Link with the last block
        if last_block_addr != 0 {
            unsafe {
                let last_block = self.get_block_mut(last_block_addr);
                last_block.next = Some(start);
                drop(last_block);
                let new_block = self.get_block_mut(start);
                new_block.prev = Some(last_block_addr);
            }
        } else {
            self.first_block = Some(start);
        }
        
        self.memory_size += size;
        self.coalesce_blocks();
        Ok(())
    }
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align().max(8);
        
        if let Some(block_addr) = self.find_free_block(size, align) {
            let mut block_size;
            let mut block_next;
            let mut block_prev;
            
            // Extract the information we need from the block first
            unsafe {
                let block = self.get_block(block_addr);
                block_size = block.size;
                block_next = block.next;
                block_prev = block.prev;
            }
            
            let data_addr = block_addr + core::mem::size_of::<Block>();
            let aligned_addr = (data_addr + align - 1) & !(align - 1);
            let padding = aligned_addr - data_addr;
            
            // Split block if necessary (if remaining space is large enough)
            if block_size >= size + padding + core::mem::size_of::<Block>() + align {
                let new_block_addr = block_addr + core::mem::size_of::<Block>() + size + padding;
                
                // Initialize new block 
                unsafe {
                    let new_block = self.get_block_mut(new_block_addr);
                    *new_block = Block {
                        start: new_block_addr,
                        size: block_size - size - padding - core::mem::size_of::<Block>(),
                        is_free: true,
                        next: block_next,
                        prev: Some(block_addr),
                    };
                }
                
                // Update original block
                unsafe {
                    let block = self.get_block_mut(block_addr);
                    block.size = size + padding;
                    block.next = Some(new_block_addr);
                    block.is_free = false;
                }
            } else {
                // Mark block as used without splitting
                unsafe {
                    let block = self.get_block_mut(block_addr);
                    block.is_free = false;
                }
            }
            
            self.used_bytes += size + padding;
            self.coalesce_blocks();
            Ok(NonNull::new(aligned_addr as *mut u8).unwrap())
        } else {
            Err(allocator::AllocError::NoMemory)
        }
    }
    
    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        let addr = pos.as_ptr() as usize;
        let mut current = self.first_block;
        let mut block_size = 0;
        let mut block_to_free = None;
        
        // Find the block containing this address
        while let Some(block_addr) = current {
            unsafe {
                let block = self.get_block(block_addr);
                let data_addr = block_addr + core::mem::size_of::<Block>();
                let block_end = data_addr + block.size;
                
                if addr >= data_addr && addr < block_end {
                    // Found the block
                    block_to_free = Some(block_addr);
                    block_size = block.size;
                    break;
                }
                
                current = block.next;
            }
        }
        
        if let Some(block_addr) = block_to_free {
            // Mark block as free
            self.used_bytes -= block_size;
            
            unsafe {
                let block = self.get_block_mut(block_addr);
                block.is_free = true;
            }
            
            // Attempt to coalesce blocks
            self.coalesce_blocks();
        }
    }
    
    fn total_bytes(&self) -> usize {
        self.memory_size
    }
    
    fn used_bytes(&self) -> usize {
        self.used_bytes
    }
    
    fn available_bytes(&self) -> usize {
        self.memory_size - self.used_bytes
    }
}

// Additional helper methods
impl LabByteAllocator {
    // Coalesce adjacent free blocks to reduce fragmentation
    fn coalesce_blocks(&mut self) {
        let mut current = self.first_block;

        while let Some(block_addr) = current {
            let block = unsafe { self.get_block(block_addr) };
            if !block.is_free {
                current = block.next;
                continue;
            }

            // Try to merge with next block
            if let Some(next_addr) = block.next {
                let next_block = unsafe { self.get_block(next_addr) };
                if next_block.is_free {
                    let next_size = next_block.size;
                    let next_next = next_block.next;

                    unsafe {
                        let block = self.get_block_mut(block_addr);
                        block.size += next_size + core::mem::size_of::<Block>();
                        block.next = next_next;
                        if let Some(next_next_addr) = next_next {
                            let next_next_block = self.get_block_mut(next_next_addr);
                            next_next_block.prev = Some(block_addr);
                        }
                    }
                    // Don't advance current, check again for further merges
                    continue;
                }
            }

            // Try to merge with previous block
            if let Some(prev_addr) = block.prev {
                let prev_block = unsafe { self.get_block(prev_addr) };
                if prev_block.is_free {
                    unsafe {
                        let block = self.get_block_mut(block_addr);
                        let block_size = block.size;
                        let block_next = block.next;

                        let prev_block = self.get_block_mut(prev_addr);
                        prev_block.size += block_size + core::mem::size_of::<Block>();
                        prev_block.next = block_next;

                        if let Some(next_addr) = block_next {
                            let next_block = self.get_block_mut(next_addr);
                            next_block.prev = Some(prev_addr);
                        }
                    }
                    // Continue with current block, as prev_block has been extended
                    continue;
                }
            }

            current = unsafe { self.get_block(block_addr).next };
        }
    }    
}