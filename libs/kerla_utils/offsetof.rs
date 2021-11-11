#[macro_export]
macro_rules! offset_of {
    ($struct:ty, $field:ident) => {{
        let dummy = core::mem::MaybeUninit::<$struct>::uninit();
        let dummy_ptr = dummy.as_ptr();
        let base_addr = dummy_ptr as usize;
        let field_addr = unsafe { &((*dummy_ptr).$field) as *const _ as usize };
        let offset = (field_addr - base_addr) as isize;
        offset
    }};
}
