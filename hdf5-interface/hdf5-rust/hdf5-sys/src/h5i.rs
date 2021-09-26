pub use self::H5I_type_t::*;

use crate::internal_prelude::*;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum H5I_type_t {
    H5I_UNINIT = -2,
    H5I_BADID = -1,
    H5I_FILE = 1,
    H5I_GROUP,
    H5I_DATATYPE,
    H5I_DATASPACE,
    H5I_DATASET,
    #[cfg(hdf5_1_12_0)]
    H5I_MAP,
    H5I_ATTR,
    #[cfg(not(hdf5_1_12_0))]
    #[cfg_attr(hdf5_1_10_2, deprecated(note = "deprecated in HDF5 1.10.2"))]
    H5I_REFERENCE,
    H5I_VFL,
    #[cfg(hdf5_1_12_0)]
    H5I_VOL,
    H5I_GENPROP_CLS,
    H5I_GENPROP_LST,
    H5I_ERROR_CLASS,
    H5I_ERROR_MSG,
    H5I_ERROR_STACK,
    #[cfg(hdf5_1_12_0)]
    H5I_SPACE_SEL_ITER,
    H5I_NTYPES,
}

#[cfg(hdf5_1_10_0)]
pub type hid_t = i64;

#[cfg(not(hdf5_1_10_0))]
pub type hid_t = c_int;

pub const H5I_INVALID_HID: hid_t = -1;

pub type H5I_free_t = Option<extern "C" fn(arg1: *mut c_void) -> herr_t>;
pub type H5I_search_func_t =
    Option<extern "C" fn(obj: *mut c_void, id: hid_t, key: *mut c_void) -> c_int>;
#[cfg(hdf5_1_12_0)]
pub type H5I_iterate_func_t = Option<extern "C" fn(id: hid_t, udata: *mut c_void) -> herr_t>;

extern "C" {
    pub fn H5Iregister(type_: H5I_type_t, object: *const c_void) -> hid_t;
    pub fn H5Iobject_verify(id: hid_t, id_type: H5I_type_t) -> *mut c_void;
    pub fn H5Iremove_verify(id: hid_t, id_type: H5I_type_t) -> *mut c_void;
    pub fn H5Iget_type(id: hid_t) -> H5I_type_t;
    pub fn H5Iget_file_id(id: hid_t) -> hid_t;
    pub fn H5Iget_name(id: hid_t, name: *mut c_char, size: size_t) -> ssize_t;
    pub fn H5Iinc_ref(id: hid_t) -> c_int;
    pub fn H5Idec_ref(id: hid_t) -> c_int;
    pub fn H5Iget_ref(id: hid_t) -> c_int;
    pub fn H5Iregister_type(
        hash_size: size_t, reserved: c_uint, free_func: H5I_free_t,
    ) -> H5I_type_t;
    pub fn H5Iclear_type(type_: H5I_type_t, force: hbool_t) -> herr_t;
    pub fn H5Idestroy_type(type_: H5I_type_t) -> herr_t;
    pub fn H5Iinc_type_ref(type_: H5I_type_t) -> c_int;
    pub fn H5Idec_type_ref(type_: H5I_type_t) -> c_int;
    pub fn H5Iget_type_ref(type_: H5I_type_t) -> c_int;
    pub fn H5Isearch(type_: H5I_type_t, func: H5I_search_func_t, key: *mut c_void) -> *mut c_void;
    #[cfg(hdf5_1_12_0)]
    pub fn H5Iiterate(type_: H5I_type_t, op: H5I_iterate_func_t, op_data: *mut c_void) -> herr_t;
    pub fn H5Inmembers(type_: H5I_type_t, num_members: *mut hsize_t) -> herr_t;
    pub fn H5Itype_exists(type_: H5I_type_t) -> htri_t;
    pub fn H5Iis_valid(id: hid_t) -> htri_t;
}
