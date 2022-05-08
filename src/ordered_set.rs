use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::slice::SliceIndex;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct OrderedSet<T> {
    data: Vec<T>,
}



impl<T: PartialEq + Sized> OrderedSet<T> {

    #[must_use]
    pub fn new() -> Self {
        OrderedSet { data: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get(&self, index: usize) -> Option<&<usize as SliceIndex<[T]>>::Output> {
        self.data.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut <usize as SliceIndex<[T]>>::Output> {
        self.data.get_mut(index)
    }

    pub fn contains(&mut self, item: &T) -> bool {
        for i in 0..self.data.len() {
            if &self.data[i] == item{
                return true;
            }
        }

        false
    }

    pub fn push(&mut self, item: T) {
        if !self.contains(&item){
            self.data.push(item);
        }
    }

    pub fn remove(&mut self, item: T) {
        if self.contains(&item){
            let index = self.data.iter().position(|x| *x == item).unwrap();
            self.data.remove(index);
        }
    }

    pub fn to_vec(&self) -> &Vec<T> {
        &self.data
    }
}




