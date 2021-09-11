//! 提供了基础的数据结构
//! CycleQueue 是一个静态不可扩容的环形队列
//! 该队列支持强行推入操作，即抛弃掉最先进入的值，推入想要的值

#![allow(dead_code)]

use crate::alloc::vec::Vec;
use generic_array::{ArrayLength, GenericArray};

pub struct CycleQueue<T, N: ArrayLength<Option<T>>> {
    data: GenericArray<Option<T>, N>,
    head: usize,
    tail: usize,
}

impl<T, N: ArrayLength<Option<T>>> CycleQueue<T, N> {
    pub fn new() -> CycleQueue<T, N> {
        CycleQueue {
            data: GenericArray::default(),
            head: 0,
            tail: 0,
        }
    }

    pub fn clean(&mut self) {
        self.tail = 0;
        self.head = 0;
    }

    pub fn length(&self) -> usize {
        (self.tail - self.head + N::to_usize()) / N::to_usize()
    }

    pub fn free_len(&self) -> usize {
        N::to_usize() - self.length() - 1
    }

    pub fn empty(&self) -> bool {
        if self.head == self.tail {
            true
        } else {
            false
        }
    }

    pub fn full(&self) -> bool {
        if (self.tail + 1) % N::to_usize() == self.head {
            true
        } else {
            false
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.empty() {
            None
        } else {
            let val = self.data[self.head].take().unwrap();
            self.head = (self.head + 1) % N::to_usize();
            Some(val)
        }
    }

    pub fn push(&mut self, val: T) -> Result<(), T> {
        if self.full() {
            Err(val)
        } else {
            self.data[self.tail] = Some(val);
            self.tail = (self.tail + 1) % N::to_usize();
            Ok(())
        }
    }

    pub fn force_push(&mut self, val: T) -> Option<T> {
        let pop;
        if self.full() {
            pop = self.pop().unwrap();
            let _ = self.push(val);
            Some(pop)
        } else {
            let _ = self.push(val);
            None
        }
    }
}

pub struct DynCycleQueue<T> {
    data: Vec<Option<T>>,
    capacity: usize,
    head: usize,
    tail: usize,
}

impl<T: Clone> DynCycleQueue<T> {
    pub fn new(capacity: usize) -> DynCycleQueue<T> {
        let mut d = Vec::new();
        d.resize(capacity, None);

        DynCycleQueue {
            data: d,
            capacity,
            head: 0,
            tail: 0,
        }
    }

    pub fn resize(&mut self, size: usize) {
        self.tail = 0;
        self.head = 0;
        self.capacity = size;
        self.data.resize(size, None)
    }

    pub fn clean(&mut self) {
        self.tail = 0;
        self.head = 0;
    }

    pub fn length(&self) -> usize {
        (self.tail - self.head + self.capacity) / self.capacity
    }

    pub fn free_len(&self) -> usize {
        self.capacity - self.length() - 1
    }

    pub fn empty(&self) -> bool {
        if self.head == self.tail {
            true
        } else {
            false
        }
    }

    pub fn full(&self) -> bool {
        if (self.tail + 1) % self.capacity == self.head {
            true
        } else {
            false
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.empty() {
            None
        } else {
            let val = self.data[self.head].take().unwrap();
            self.head = (self.head + 1) % self.capacity;
            Some(val)
        }
    }

    pub fn push(&mut self, val: T) -> Result<(), T> {
        if self.full() {
            Err(val)
        } else {
            self.data[self.tail] = Some(val);
            self.tail = (self.tail + 1) % self.capacity;
            Ok(())
        }
    }

    pub fn force_push(&mut self, val: T) -> Option<T> {
        let pop;
        if self.full() {
            pop = self.pop().unwrap();
            let _ = self.push(val);
            Some(pop)
        } else {
            let _ = self.push(val);
            None
        }
    }
}
