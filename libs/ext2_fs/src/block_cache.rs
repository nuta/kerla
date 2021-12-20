use spin::Mutex;

use alloc::{sync::Arc, collections::VecDeque};
use lazy_static::*;
use crate::{BLOCK_SIZE, BlockDevice, error::Result};

const BLOCK_CACHE_SIZE: usize = 16;

pub struct BlockCache {
    cache: [u8; BLOCK_SIZE],
    block_id: usize,
    block_device: Arc<dyn BlockDevice>,
    modified: bool
}

pub struct BlockCacheManager {
    queue: VecDeque<(usize, Arc<Mutex<BlockCache>>)>
}

impl BlockCache {
    pub fn new(block_id: usize, block_device: Arc<dyn BlockDevice>) -> Result<Self> {
        let mut cache = [0u8; BLOCK_SIZE];
        block_device.read_block(block_id,&mut cache)?;
        Ok(Self {
            cache,
            block_id,
            block_device,
            modified: false
        })
    }

    pub fn addr_of_offset(&self, offset: usize) -> usize {
        &self.cache[offset] as *const _ as usize
    }

    pub fn get_ref<T>(&self, offset: usize) -> &T where T:Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offset);
        unsafe {
            &*(addr as *const T)
        }
    }

pub fn get_mut<T>(&mut self, offset: usize) -> &T where T:Sized {
        let type_size = core::mem::size_of::<T>();
        assert!(offset + type_size <= BLOCK_SIZE);
        let addr = self.addr_of_offset(offset);
        unsafe {
            &mut *(addr as *mut T)
        }
    }

    pub fn sync(&mut self) {
        if self.modified {
            self.modified = false;
            self.block_device.write_block(self.block_id, &self.cache).unwrap();
        }
    }

}

impl Drop for BlockCache {
    fn drop(&mut self) {
        self.sync();
    }
}

impl BlockCacheManager {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new()
        }
    }
    pub fn get_block_cache(
        &mut self,
        block_id: usize,
        block_device: Arc<dyn BlockDevice>,
    ) -> Arc<Mutex<BlockCache>> {
        if let Some(pair) = self.queue
            .iter()
            .find(|pair| pair.0 == block_id) {
                Arc::clone(&pair.1)
        } else {
            if self.queue.len() == BLOCK_CACHE_SIZE {
                if let Some((idx, _)) = self.queue
                    .iter()
                    .enumerate()
                    .find(|(_, pair)| Arc::strong_count(&pair.1) == 1) {
                    self.queue.drain(idx..=idx);
                } else {
                    panic!("Run out of BlockCache!");
                }
            }
            let block_cache = Arc::new(Mutex::new(
                BlockCache::new(block_id, Arc::clone(&block_device)).unwrap()
            ));
            self.queue.push_back((block_id, Arc::clone(&block_cache)));
            block_cache
        }
    }
}

lazy_static!  {
    pub static ref BLOCK_CACHE_MANAGER: Mutex<BlockCacheManager> = Mutex::new(
        BlockCacheManager::new()
    );
}

pub fn get_block_cache(
    block_id: usize,
    block_device: Arc<dyn BlockDevice>
) -> Arc<Mutex<BlockCache>> {
    BLOCK_CACHE_MANAGER.lock().get_block_cache(block_id, block_device)
}

pub fn block_cache_sync_all() {
    let manager = BLOCK_CACHE_MANAGER.lock();
    for (_, cache) in manager.queue.iter() {
        cache.lock().sync();
    }
}