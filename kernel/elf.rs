use crate::arch::UserVAddr;
use crate::result::Result;
use core::{intrinsics::transmute, slice::from_raw_parts};
use goblin::elf64::header::Header;
use goblin::elf64::program_header::ProgramHeader;

pub struct Elf<'a> {
    header: &'a Header,
    program_headers: &'a [ProgramHeader],
}

impl<'a> Elf<'a> {
    pub fn parse(buf: &'a [u8]) -> Elf<'a> {
        // TODO: Check the size of `buf`
        // TODO: Check magic, e_machine, etc.
        let header: &Header = unsafe { &*(buf.as_ptr() as *const Header) };
        let program_headers = unsafe {
            from_raw_parts(
                &buf[header.e_phoff as usize] as *const _ as *const ProgramHeader,
                header.e_phnum as usize,
            )
        };
        Elf {
            header,
            program_headers,
        }
    }

    pub fn entry(&self) -> Result<UserVAddr> {
        UserVAddr::new(self.header.e_entry as usize)
    }

    pub fn program_headers(&self) -> &[ProgramHeader] {
        &self.program_headers
    }
}
