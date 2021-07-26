pub use self::H5R_type_t::*;

use crate::internal_prelude::*;

use crate::h5o::H5O_type_t;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
#[cfg(not(hdf5_1_12_0))]
pub enum H5R_type_t {
    H5R_BADTYPE = -1,
    H5R_OBJECT = 0,
    H5R_DATASET_REGION = 1,
    H5R_MAXTYPE = 2,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
#[cfg(hdf5_1_12_0)]
pub enum H5R_type_t {
    H5R_BADTYPE = -1,
    H5R_OBJECT1 = 0,
    H5R_DATASET_REGION1 = 1,
    H5R_OBJECT2 = 2,
    H5R_DATASET_REGION2 = 3,
    H5R_ATTR = 4,
    H5R_MAXTYPE = 5,
}

pub type hobj_ref_t = haddr_t;
pub type hdset_reg_ref_t = [c_uchar; 12usize];

#[cfg(not(hdf5_1_10_0))]
extern "C" {
    pub fn H5Rdereference(dataset: hid_t, ref_type: H5R_type_t, ref_: *const c_void) -> hid_t;
}

extern "C" {
    pub fn H5Rcreate(
        ref_: *mut c_void, loc_id: hid_t, name: *const c_char, ref_type: H5R_type_t,
        space_id: hid_t,
    ) -> herr_t;
    pub fn H5Rget_region(dataset: hid_t, ref_type: H5R_type_t, ref_: *const c_void) -> hid_t;
    pub fn H5Rget_obj_type2(
        id: hid_t, ref_type: H5R_type_t, ref_: *const c_void, obj_type: *mut H5O_type_t,
    ) -> herr_t;
    pub fn H5Rget_name(
        loc_id: hid_t, ref_type: H5R_type_t, ref_: *const c_void, name: *mut c_char, size: size_t,
    ) -> ssize_t;
}

#[cfg(hdf5_1_10_0)]
extern "C" {
    #[deprecated(note = "deprecated in HDF5 1.10.0, use H5Rdereference2()")]
    pub fn H5Rdereference1(obj_id: hid_t, ref_type: H5R_type_t, ref_: *const c_void) -> hid_t;
    pub fn H5Rdereference2(
        obj_id: hid_t, oapl_id: hid_t, ref_type: H5R_type_t, ref_: *const c_void,
    ) -> hid_t;
}

#[cfg(hdf5_1_10_0)]
pub use self::H5Rdereference1 as H5Rdereference;

#[cfg(hdf5_1_12_0)]
pub const H5R_REF_BUF_SIZE: usize = 64;

#[cfg(hdf5_1_12_0)]
#[repr(C)]
#[derive(Copy, Clone)]
pub union H5R_ref_t_u {
    __data: [u8; H5R_REF_BUF_SIZE],
    align: i64,
}

#[cfg(hdf5_1_12_0)]
impl Default for H5R_ref_t_u {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

#[cfg(hdf5_1_12_0)]
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct H5R_ref_t {
    u: H5R_ref_t_u,
}

#[cfg(hdf5_1_12_0)]
extern "C" {
    pub fn H5Rcopy(src_ref_ptr: *const H5R_ref_t, dst_ref_ptr: *mut H5R_ref_t) -> herr_t;
    pub fn H5Rcreate_attr(
        loc_id: hid_t, name: *const c_char, attr_name: *const c_char, oapl_id: hid_t,
        ref_ptr: *mut H5R_ref_t,
    ) -> herr_t;
    pub fn H5Rcreate_object(
        loc_id: hid_t, name: *const c_char, oapl_id: hid_t, ref_ptr: *mut H5R_ref_t,
    ) -> herr_t;
    pub fn H5Rcreate_region(
        loc_id: hid_t, name: *const c_char, space_id: hid_t, oapl_id: hid_t,
        ref_ptr: *mut H5R_ref_t,
    ) -> herr_t;
    pub fn H5Rdestroy(ref_ptr: *mut H5R_ref_t) -> herr_t;
    pub fn H5Requal(ref1_ptr: *const H5R_ref_t, ref2_ptr: *const H5R_ref_t) -> htri_t;
    pub fn H5Rget_attr_name(ref_ptr: *const H5R_ref_t, name: *mut c_char, size: size_t) -> ssize_t;
    pub fn H5Rget_file_name(ref_ptr: *const H5R_ref_t, name: *mut c_char, size: size_t) -> ssize_t;
    pub fn H5Rget_obj_name(
        ref_ptr: *const H5R_ref_t, rapl_id: hid_t, name: *mut c_char, size: size_t,
    ) -> ssize_t;
    pub fn H5Rget_obj_type3(
        ref_ptr: *const H5R_ref_t, rapl_id: hid_t, obj_type: *mut H5O_type_t,
    ) -> herr_t;
    pub fn H5Rget_type(ref_ptr: *const H5R_ref_t) -> H5R_type_t;
    pub fn H5Ropen_attr(ref_ptr: *const H5R_ref_t, rapl_id: hid_t, aapl_id: hid_t) -> hid_t;
    pub fn H5Ropen_object(ref_ptr: *const H5R_ref_t, rapl_id: hid_t, oapl_id: hid_t) -> hid_t;
    pub fn H5Ropen_region(ref_ptr: *const H5R_ref_t, rapl_id: hid_t, oapl_id: hid_t) -> hid_t;
}
