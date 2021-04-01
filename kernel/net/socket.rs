use bitflags::bitflags;

bitflags! {
    pub struct RecvFromFlags: i32 {
        // TODO:
        const _NOT_IMPLEMENTED = 0;
    }
}

bitflags! {
    pub struct SendToFlags: i32 {
        // TODO:
        const _NOT_IMPLEMENTED = 0;
        const MSG_NOSIGNAL = 0x4000;
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Ipv4Address(pub [u8; 4]);

impl From<u32> for Ipv4Address {
    fn from(value: u32) -> Ipv4Address {
        Ipv4Address([
            ((value >> 24) & 0xff) as u8,
            ((value >> 16) & 0xff) as u8,
            ((value >> 8) & 0xff) as u8,
            (value & 0xff) as u8,
        ])
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum IpAddress {
    Unspecified,
    Ipv4(Ipv4Address),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Endpoint {
    pub addr: IpAddress,
    pub port: u16,
}
