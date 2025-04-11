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
    
    // Find a free block with first-fit strategy
    fn find_free_block(&self, size: usize, align: usize) -> Option<usize> {
        let mut current = self.first_block;
        
        while let Some(block_addr) = current {
            let block = unsafe { self.get_block(block_addr) };
            
            if block.is_free {
                let data_addr = block_addr + core::mem::size_of::<Block>();
                let aligned_addr = (data_addr + align - 1) & !(align - 1);
                let padding = aligned_addr - data_addr;
                
                if block.size >= size + padding {
                    return Some(block_addr);
                }
            }
            
            current = block.next;
        }
        
        None
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
            };
        }
        
        self.first_block = Some(start);
    }
    
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
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
        
        // Create new block
        unsafe {
            let new_block = self.get_block_mut(start);
            *new_block = Block {
                start,
                size: size - core::mem::size_of::<Block>(),
                is_free: true,
                next: None,
            };
        }
        
        // Link with the last block
        if last_block_addr != 0 {
            unsafe {
                let last_block = self.get_block_mut(last_block_addr);
                last_block.next = Some(start);
            }
        } else {
            self.first_block = Some(start);
        }
        
        self.memory_size += size;
        Ok(())
    }
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();
        
        if let Some(block_addr) = self.find_free_block(size, align) {
            let mut block_size;
            let mut block_next;
            
            // Extract the information we need from the block first
            unsafe {
                let block = self.get_block(block_addr);
                block_size = block.size;
                block_next = block.next;
            }
            
            let data_addr = block_addr + core::mem::size_of::<Block>();
            let aligned_addr = (data_addr + align - 1) & !(align - 1);
            let padding = aligned_addr - data_addr;
            
            // Split block if necessary (if remaining space is large enough)
            if block_size >= size + padding + core::mem::size_of::<Block>() + 8 {
                let new_block_addr = block_addr + core::mem::size_of::<Block>() + size + padding;
                
                // Initialize new block 
                unsafe {
                    let new_block = self.get_block_mut(new_block_addr);
                    *new_block = Block {
                        start: new_block_addr,
                        size: block_size - size - padding - core::mem::size_of::<Block>(),
                        is_free: true,
                        next: block_next,
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
            let is_free;
            let next_block;
            
            unsafe {
                let block = self.get_block(block_addr);
                is_free = block.is_free;
                next_block = block.next;
            }
            
            if is_free {
                // Check if next block is also free
                if let Some(next_addr) = next_block {
                    let next_is_free;
                    let next_size;
                    let next_next;
                    
                    unsafe {
                        let next = self.get_block(next_addr);
                        next_is_free = next.is_free;
                        next_size = next.size;
                        next_next = next.next;
                    }
                    
                    if next_is_free {
                        // Coalesce with next block
                        unsafe {
                            let block = self.get_block_mut(block_addr);
                            block.size += next_size + core::mem::size_of::<Block>();
                            block.next = next_next;
                        }
                        // Don't advance current, as we might be able to coalesce with the next block too
                        continue;
                    }
                }
            }
            
            current = next_block;
        }
    }
}