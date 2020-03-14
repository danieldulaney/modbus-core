#![no_std]

pub mod bit_pack;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coil {
    On,
    Off,
}
