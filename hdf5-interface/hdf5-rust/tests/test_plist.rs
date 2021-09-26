use std::mem;
use std::str::FromStr;

use hdf5::dataset::*;
use hdf5::file::*;
use hdf5::plist::*;

macro_rules! test_pl {
    ($ty:ident, $field:ident ($($arg:expr),+): $($name:ident=$value:expr),+) => (
        test_pl!($ty, $field ($($arg,)+): $($name=$value,)+)
    );

    ($ty:ident, $field:ident ($($arg:expr,)+): $($name:ident=$value:expr,)+) => ({
        let mut b = $ty::build();
        b.$field($($arg,)+);
        let fapl = b.finish()?;
        $(assert_eq!(fapl.$field().$name, $value);)+
        paste::paste! { $(assert_eq!(fapl.[<get_ $field>]()?.$name, $value);)+ }
    });

    ($ty:ident, $field:ident: $($name:ident=$value:expr),+) => (
        test_pl!($ty, $field: $($name=$value,)+)
    );

    ($ty:ident, $field:ident: $($name:ident=$value:expr,)+) => ({
        test_pl!($ty, $field ($($value,)+): $($name=$value,)+)
    });

    ($ty:ident, $field:ident ($arg:expr): $value:expr) => ({
        let mut b = $ty::build();
        b.$field($arg);
        let fapl = b.finish()?;
        assert_eq!(fapl.$field(), $value);
        paste::paste! { assert_eq!(fapl.[<get_ $field>]()?, $value); }
    });

    ($ty:ident, $field:ident: $value:expr) => ({
        test_pl!($ty, $field ($value): $value)
    });
}

macro_rules! test_pl_common {
    ($cls:ident, $plc:expr, $func:expr) => {
        let pl_default = $cls::try_new()?;
        assert_eq!(pl_default.class()?, $plc);
        assert_eq!(pl_default, pl_default);

        assert!(format!("{:?}", pl_default).starts_with(&format!("{:?}", $plc)));

        let mut b = $cls::build();
        let pl = $func(&mut b)?;
        assert_eq!(pl.class()?, $plc);
        assert_eq!(pl, pl);
        assert_ne!(pl, pl_default);

        let pl2 = pl.copy();
        assert_eq!(pl2.class()?, $plc);
        assert_eq!(pl2, pl);
        assert_ne!(pl2, pl_default);
    };
}

macro_rules! check_matches {
    ($e:expr, $o:expr, $($p:tt)+) => (
        match $e {
            $($p)+ => $o,
            ref e => panic!("assertion failed: `{:?}` does not match `{}`", e, stringify!($($p)+)),
        }
    )
}

type FC = FileCreate;
type FCB = FileCreateBuilder;

#[test]
fn test_fcpl_common() -> hdf5::Result<()> {
    test_pl_common!(FC, PropertyListClass::FileCreate, |b: &mut FCB| b.userblock(2048).finish());
    Ok(())
}

#[test]
fn test_fcpl_sizes() -> hdf5::Result<()> {
    use hdf5_sys::h5::hsize_t;
    let fcpl = FileCreate::try_new()?;
    assert_eq!(fcpl.sizes().sizeof_addr, mem::size_of::<hsize_t>());
    assert_eq!(fcpl.sizes().sizeof_size, mem::size_of::<hsize_t>());
    Ok(())
}

#[test]
fn test_fcpl_set_userblock() -> hdf5::Result<()> {
    test_pl!(FC, userblock: 0);
    test_pl!(FC, userblock: 4096);
    Ok(())
}

#[test]
fn test_fcpl_set_sym_k() -> hdf5::Result<()> {
    test_pl!(FC, sym_k: tree_rank = 17, node_size = 5);
    test_pl!(FC, sym_k: tree_rank = 18, node_size = 6);
    Ok(())
}

#[test]
fn test_fcpl_set_istore_k() -> hdf5::Result<()> {
    test_pl!(FC, istore_k: 33);
    test_pl!(FC, istore_k: 123);
    Ok(())
}

#[test]
fn test_fcpl_set_shared_mesg_change() -> hdf5::Result<()> {
    test_pl!(FC, shared_mesg_phase_change: max_list = 51, min_btree = 41);
    test_pl!(FC, shared_mesg_phase_change: max_list = 52, min_btree = 42);
    Ok(())
}

#[test]
fn test_fcpl_set_shared_mesg_indexes() -> hdf5::Result<()> {
    let idx = vec![SharedMessageIndex {
        message_types: SharedMessageType::ATTRIBUTE,
        min_message_size: 16,
    }];
    test_pl!(FC, shared_mesg_indexes(&idx): idx);
    let idx = vec![];
    test_pl!(FC, shared_mesg_indexes(&idx): idx);
    Ok(())
}

#[test]
fn test_fcpl_obj_track_times() -> hdf5::Result<()> {
    assert_eq!(FC::try_new()?.get_obj_track_times()?, true);
    assert_eq!(FC::try_new()?.obj_track_times(), true);
    test_pl!(FC, obj_track_times: true);
    test_pl!(FC, obj_track_times: false);
    Ok(())
}

#[test]
fn test_fcpl_attr_phase_change() -> hdf5::Result<()> {
    assert_eq!(FC::try_new()?.get_attr_phase_change()?, AttrPhaseChange::default());
    assert_eq!(FC::try_new()?.attr_phase_change(), AttrPhaseChange::default());
    let pl = FCB::new().attr_phase_change(34, 21).finish()?;
    let expected = AttrPhaseChange { max_compact: 34, min_dense: 21 };
    assert_eq!(pl.get_attr_phase_change()?, expected);
    assert_eq!(pl.attr_phase_change(), expected);
    assert_eq!(FCB::from_plist(&pl)?.finish()?.get_attr_phase_change()?, expected);
    assert!(FCB::new().attr_phase_change(12, 34).finish().is_err());
    Ok(())
}

#[test]
fn test_fcpl_attr_creation_order() -> hdf5::Result<()> {
    assert_eq!(FC::try_new()?.get_attr_creation_order()?.bits(), 0);
    assert_eq!(FC::try_new()?.attr_creation_order().bits(), 0);
    test_pl!(FC, attr_creation_order: AttrCreationOrder::TRACKED);
    test_pl!(FC, attr_creation_order: AttrCreationOrder::TRACKED | AttrCreationOrder::INDEXED);
    assert!(FCB::new().attr_creation_order(AttrCreationOrder::INDEXED).finish().is_err());
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_1)]
fn test_fcpl_set_file_space_page_size() -> hdf5::Result<()> {
    test_pl!(FC, file_space_page_size: 512);
    test_pl!(FC, file_space_page_size: 999);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_1)]
fn test_fcpl_set_file_space_strategy() -> hdf5::Result<()> {
    test_pl!(FC, file_space_strategy: FileSpaceStrategy::PageAggregation);
    test_pl!(FC, file_space_strategy: FileSpaceStrategy::None);
    let fsm = FileSpaceStrategy::FreeSpaceManager { paged: true, persist: true, threshold: 123 };
    test_pl!(FC, file_space_strategy: fsm);
    Ok(())
}

type FA = FileAccess;
type FAB = FileAccessBuilder;

#[test]
fn test_fapl_common() -> hdf5::Result<()> {
    test_pl_common!(FA, PropertyListClass::FileAccess, |b: &mut FAB| b.sieve_buf_size(8).finish());
    Ok(())
}

#[test]
fn test_fapl_driver_sec2() -> hdf5::Result<()> {
    let mut b = FileAccess::build();
    b.sec2();
    check_matches!(b.finish()?.get_driver()?, (), FileDriver::Sec2);
    Ok(())
}

#[test]
fn test_fapl_driver_stdio() -> hdf5::Result<()> {
    let mut b = FileAccess::build();
    b.stdio();
    check_matches!(b.finish()?.get_driver()?, (), FileDriver::Stdio);
    Ok(())
}

#[test]
fn test_fapl_driver_log() -> hdf5::Result<()> {
    let mut b = FileAccess::build();

    b.log();
    check_matches!(b.finish()?.get_driver()?, (), FileDriver::Log);

    b.log_options(Some("abc"), LogFlags::TRUNCATE, 123);
    check_matches!(b.finish()?.get_driver()?, (), FileDriver::Log);

    Ok(())
}

#[test]
fn test_fapl_driver_core() -> hdf5::Result<()> {
    let mut b = FileAccess::build();

    b.core();
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Core(d));
    assert_eq!(d.increment, 1024 * 1024);
    assert_eq!(d.filebacked, false);
    #[cfg(hdf5_1_8_13)]
    assert_eq!(d.write_tracking, 0);

    b.core_options(123, true);
    #[cfg(hdf5_1_8_13)]
    b.write_tracking(456);
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Core(d));
    assert_eq!(d.increment, 123);
    assert_eq!(d.filebacked, true);
    #[cfg(hdf5_1_8_13)]
    assert_eq!(d.write_tracking, 456);

    b.core_filebacked(false);
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Core(d));
    assert_eq!(d.increment, CoreDriver::default().increment);
    assert_eq!(d.filebacked, false);

    b.core_filebacked(true);
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Core(d));
    assert_eq!(d.increment, CoreDriver::default().increment);
    assert_eq!(d.filebacked, true);

    Ok(())
}

#[test]
fn test_fapl_driver_family() -> hdf5::Result<()> {
    let mut b = FileAccess::build();

    b.family();
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Family(d));
    assert_eq!(d.member_size, 0);

    b.family_options(123);
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Family(d));
    assert_eq!(d.member_size, 123);

    Ok(())
}

#[test]
fn test_fapl_driver_multi() -> hdf5::Result<()> {
    let mut b = FileAccess::build();

    b.multi();
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Multi(d));
    assert_eq!(d, MultiDriver::default());

    let files = vec![
        MultiFile::new("foo", 1 << 20),
        MultiFile::new("bar", 1 << 30),
        MultiFile::new("baz", 1 << 40),
        MultiFile::new("qwe", 1 << 50),
    ];
    let layout = MultiLayout {
        mem_super: 0,
        mem_btree: 1,
        mem_draw: 2,
        mem_gheap: 3,
        mem_lheap: 3,
        mem_object: 2,
    };
    b.multi_options(&files, &layout, true);
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Multi(d));
    assert_eq!(d.files, files);
    assert_eq!(d.layout, layout);
    assert_eq!(d.relax, true);

    Ok(())
}

#[test]
fn test_fapl_driver_split() -> hdf5::Result<()> {
    let mut b = FileAccess::build();

    b.split();
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Split(d));
    assert_eq!(d, SplitDriver::default());

    b.split_options(".foo", ".bar");
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Split(d));
    assert_eq!(&d.meta_ext, ".foo");
    assert_eq!(&d.raw_ext, ".bar");

    Ok(())
}

#[test]
#[cfg(feature = "mpio")]
fn test_fapl_driver_mpio() -> hdf5::Result<()> {
    use std::os::raw::c_int;
    use std::ptr;

    use mpi_sys::{MPI_Comm_compare, MPI_Init, MPI_Initialized, MPI_CONGRUENT, RSMPI_COMM_WORLD};

    let mut initialized: c_int = 1;
    unsafe { MPI_Initialized(&mut initialized) };
    if initialized == 0 {
        unsafe { MPI_Init(ptr::null_mut(), ptr::null_mut()) };
    }
    let world_comm = unsafe { RSMPI_COMM_WORLD };

    let mut b = FileAccess::build();
    b.mpio(world_comm, None);

    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Mpio(d));
    let mut cmp = mem::MaybeUninit::uninit();
    unsafe { MPI_Comm_compare(d.comm, world_comm, cmp.as_mut_ptr()) };
    assert_eq!(unsafe { cmp.assume_init() }, MPI_CONGRUENT as _);

    Ok(())
}

#[test]
#[cfg(h5_have_direct)]
fn test_fapl_driver_direct() -> hdf5::Result<()> {
    let mut b = FileAccess::build();

    b.direct();
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Direct(d));
    assert_eq!(d, DirectDriver::default());

    b.direct_options(100, 200, 400);
    let d = check_matches!(b.finish()?.get_driver()?, d, FileDriver::Direct(d));
    assert_eq!(d.alignment, 100);
    assert_eq!(d.block_size, 200);
    assert_eq!(d.cbuf_size, 400);

    Ok(())
}

#[test]
fn test_fapl_set_alignment() -> hdf5::Result<()> {
    test_pl!(FA, alignment: threshold = 1, alignment = 1);
    test_pl!(FA, alignment: threshold = 0, alignment = 32);
    Ok(())
}

#[test]
fn test_fapl_set_fclose_degree() -> hdf5::Result<()> {
    test_pl!(FA, fclose_degree: FileCloseDegree::Default);
    test_pl!(FA, fclose_degree: FileCloseDegree::Weak);
    test_pl!(FA, fclose_degree: FileCloseDegree::Semi);
    test_pl!(FA, fclose_degree: FileCloseDegree::Strong);
    Ok(())
}

#[test]
fn test_fapl_set_chunk_cache() -> hdf5::Result<()> {
    test_pl!(FA, chunk_cache: nslots = 1, nbytes = 100, w0 = 0.0);
    test_pl!(FA, chunk_cache: nslots = 10, nbytes = 200, w0 = 0.5);
    test_pl!(FA, chunk_cache: nslots = 20, nbytes = 300, w0 = 1.0);
    Ok(())
}

#[test]
fn test_fapl_set_meta_block_size() -> hdf5::Result<()> {
    test_pl!(FA, meta_block_size: 0);
    test_pl!(FA, meta_block_size: 123);
    Ok(())
}

#[test]
fn test_fapl_set_sieve_buf_size() -> hdf5::Result<()> {
    test_pl!(FA, sieve_buf_size: 42);
    test_pl!(FA, sieve_buf_size: 4096);
    Ok(())
}

#[test]
fn test_fapl_set_gc_references() -> hdf5::Result<()> {
    test_pl!(FA, gc_references: true);
    test_pl!(FA, gc_references: false);
    Ok(())
}

#[test]
fn test_fapl_set_small_data_block_size() -> hdf5::Result<()> {
    test_pl!(FA, small_data_block_size: 0);
    test_pl!(FA, small_data_block_size: 123);
    Ok(())
}

#[test]
fn test_fapl_set_mdc_config() -> hdf5::Result<()> {
    let mdc_config_1 = MetadataCacheConfig {
        rpt_fcn_enabled: false,
        open_trace_file: false,
        close_trace_file: false,
        trace_file_name: "".into(),
        evictions_enabled: true,
        set_initial_size: true,
        initial_size: 1 << 22,
        min_clean_fraction: 0.30000001192092890,
        max_size: 1 << 26,
        min_size: 1 << 21,
        epoch_length: 60_000,
        incr_mode: CacheIncreaseMode::Threshold,
        lower_hr_threshold: 0.8999999761581420,
        increment: 3.0,
        apply_max_increment: true,
        max_increment: 1 << 23,
        flash_incr_mode: FlashIncreaseMode::AddSpace,
        flash_multiple: 2.0,
        flash_threshold: 0.5,
        decr_mode: CacheDecreaseMode::AgeOutWithThreshold,
        upper_hr_threshold: 0.9990000128746030,
        decrement: 0.8999999761581420,
        apply_max_decrement: true,
        max_decrement: 1 << 21,
        epochs_before_eviction: 4,
        apply_empty_reserve: true,
        empty_reserve: 0.10000000149011610,
        dirty_bytes_threshold: 1 << 19,
        metadata_write_strategy: MetadataWriteStrategy::Distributed,
    };

    let mdc_config_2 = MetadataCacheConfig {
        rpt_fcn_enabled: true,
        open_trace_file: true,
        close_trace_file: true,
        trace_file_name: "abc".into(),
        evictions_enabled: false,
        set_initial_size: false,
        initial_size: 1 << 23,
        min_clean_fraction: 0.30000001192092899,
        max_size: 1 << 27,
        min_size: 1 << 22,
        epoch_length: 70_000,
        incr_mode: CacheIncreaseMode::Off,
        lower_hr_threshold: 0.8999999761581499,
        increment: 4.0,
        apply_max_increment: false,
        max_increment: 1 << 24,
        flash_incr_mode: FlashIncreaseMode::Off,
        flash_multiple: 3.0,
        flash_threshold: 0.6,
        decr_mode: CacheDecreaseMode::Off,
        upper_hr_threshold: 0.9990000128746099,
        decrement: 0.8999999761581499,
        apply_max_decrement: false,
        max_decrement: 1 << 22,
        epochs_before_eviction: 5,
        apply_empty_reserve: false,
        empty_reserve: 0.10000000149011699,
        dirty_bytes_threshold: 1 << 20,
        metadata_write_strategy: MetadataWriteStrategy::ProcessZeroOnly,
    };

    test_pl!(FA, mdc_config(&mdc_config_1): mdc_config_1);
    test_pl!(FA, mdc_config(&mdc_config_2): mdc_config_2);

    Ok(())
}

#[test]
#[cfg(hdf5_1_8_7)]
fn test_fapl_set_elink_file_cache_size() -> hdf5::Result<()> {
    test_pl!(FA, elink_file_cache_size: 0);
    test_pl!(FA, elink_file_cache_size: 17);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_0)]
fn test_fapl_set_metadata_read_attempts() -> hdf5::Result<()> {
    test_pl!(FA, metadata_read_attempts: 1);
    test_pl!(FA, metadata_read_attempts: 17);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_0)]
fn test_fapl_set_mdc_log_options() -> hdf5::Result<()> {
    test_pl!(FA, mdc_log_options: is_enabled = true, location = "abc", start_on_access = false,);
    test_pl!(FA, mdc_log_options: is_enabled = false, location = "", start_on_access = true,);
    Ok(())
}

#[test]
#[cfg(all(hdf5_1_10_0, feature = "mpio"))]
fn test_fapl_set_all_coll_metadata_ops() -> hdf5::Result<()> {
    test_pl!(FA, all_coll_metadata_ops: true);
    test_pl!(FA, all_coll_metadata_ops: false);
    Ok(())
}

#[test]
#[cfg(all(hdf5_1_10_0, feature = "mpio"))]
fn test_fapl_set_coll_metadata_write() -> hdf5::Result<()> {
    test_pl!(FA, coll_metadata_write: true);
    test_pl!(FA, coll_metadata_write: false);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_2)]
fn test_fapl_set_libver_bounds() -> hdf5::Result<()> {
    test_pl!(FA, libver_bounds: low = LibraryVersion::Earliest, high = LibraryVersion::V18);
    test_pl!(FA, libver_bounds: low = LibraryVersion::Earliest, high = LibraryVersion::V110);
    test_pl!(FA, libver_bounds: low = LibraryVersion::V18, high = LibraryVersion::V18);
    test_pl!(FA, libver_bounds: low = LibraryVersion::V18, high = LibraryVersion::V110);
    test_pl!(FA, libver_bounds: low = LibraryVersion::V110, high = LibraryVersion::V110);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_1)]
fn test_fapl_set_page_buffer_size() -> hdf5::Result<()> {
    test_pl!(FA, page_buffer_size: buf_size = 0, min_meta_perc = 0, min_raw_perc = 0);
    test_pl!(FA, page_buffer_size: buf_size = 0, min_meta_perc = 7, min_raw_perc = 9);
    test_pl!(FA, page_buffer_size: buf_size = 3, min_meta_perc = 0, min_raw_perc = 5);
    Ok(())
}

#[test]
#[cfg(all(hdf5_1_10_1, not(h5_have_parallel)))]
fn test_fapl_set_evict_on_close() -> hdf5::Result<()> {
    test_pl!(FA, evict_on_close: true);
    test_pl!(FA, evict_on_close: false);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_1)]
fn test_fapl_set_mdc_image_config() -> hdf5::Result<()> {
    test_pl!(FA, mdc_image_config: generate_image = true);
    test_pl!(FA, mdc_image_config: generate_image = false);
    Ok(())
}

type DA = DatasetAccess;
type DAB = DatasetAccessBuilder;

#[test]
fn test_dapl_common() -> hdf5::Result<()> {
    test_pl_common!(DA, PropertyListClass::DatasetAccess, |b: &mut DAB| b
        .chunk_cache(100, 200, 0.5)
        .finish());
    Ok(())
}

#[test]
#[cfg(hdf5_1_8_17)]
fn test_dapl_set_efile_prefix() -> hdf5::Result<()> {
    assert_eq!(DA::try_new()?.get_efile_prefix().unwrap(), "".to_owned());
    assert_eq!(DA::try_new()?.efile_prefix(), "".to_owned());
    let mut b = DA::build();
    b.efile_prefix("foo");
    assert_eq!(b.finish()?.get_efile_prefix()?, "foo".to_owned());
    Ok(())
}

#[test]
fn test_dapl_set_chunk_cache() -> hdf5::Result<()> {
    test_pl!(DA, chunk_cache: nslots = 1, nbytes = 100, w0 = 0.0);
    test_pl!(DA, chunk_cache: nslots = 10, nbytes = 200, w0 = 0.5);
    test_pl!(DA, chunk_cache: nslots = 20, nbytes = 300, w0 = 1.0);
    Ok(())
}

#[test]
#[cfg(all(hdf5_1_10_0, feature = "mpio"))]
fn test_dapl_set_all_coll_metadata_ops() -> hdf5::Result<()> {
    test_pl!(DA, all_coll_metadata_ops: true);
    test_pl!(DA, all_coll_metadata_ops: false);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_0)]
fn test_dapl_set_virtual_view() -> hdf5::Result<()> {
    test_pl!(DA, virtual_view: VirtualView::FirstMissing);
    test_pl!(DA, virtual_view: VirtualView::LastAvailable);
    Ok(())
}

#[test]
#[cfg(hdf5_1_10_0)]
fn test_dapl_set_virtual_printf_gap() -> hdf5::Result<()> {
    test_pl!(DA, virtual_printf_gap: 0);
    test_pl!(DA, virtual_printf_gap: 123);
    Ok(())
}

type DC = DatasetCreate;
type DCB = DatasetCreateBuilder;

#[test]
fn test_dcpl_common() -> hdf5::Result<()> {
    test_pl_common!(DC, PropertyListClass::DatasetCreate, |b: &mut DCB| b
        .layout(Layout::Compact)
        .finish());
    Ok(())
}

#[test]
fn test_dcpl_set_chunk() -> hdf5::Result<()> {
    assert!(DC::try_new()?.get_chunk()?.is_none());
    assert_eq!(DCB::new().chunk(&[3, 7]).finish()?.get_chunk()?, Some(vec![3, 7]));
    assert_eq!(DCB::new().chunk((3, 7)).finish()?.chunk(), Some(vec![3, 7]));
    let mut b = DCB::new().chunk([3, 7]).clone();
    assert_eq!(b.layout(Layout::Contiguous).finish()?.layout(), Layout::Chunked);
    assert_eq!(b.layout(Layout::Compact).finish()?.layout(), Layout::Chunked);
    #[cfg(hdf5_1_10_0)]
    assert_eq!(b.layout(Layout::Virtual).finish()?.layout(), Layout::Chunked);
    assert!(b.no_chunk().finish()?.chunk().is_none());
    assert!(DCB::new().layout(Layout::Contiguous).finish()?.get_chunk()?.is_none());
    assert!(DCB::new().layout(Layout::Compact).finish()?.get_chunk()?.is_none());
    #[cfg(hdf5_1_10_0)]
    assert!(DCB::new().layout(Layout::Virtual).finish()?.get_chunk()?.is_none());
    assert_eq!(DCB::new().layout(Layout::Chunked).finish()?.get_chunk()?, Some(vec![]));
    Ok(())
}

#[test]
fn test_dcpl_set_layout() -> hdf5::Result<()> {
    check_matches!(DC::try_new()?.get_layout()?, (), Layout::Contiguous);
    test_pl!(DC, layout: Layout::Contiguous);
    test_pl!(DC, layout: Layout::Compact);
    test_pl!(DC, layout: Layout::Chunked);
    #[cfg(hdf5_1_10_0)]
    test_pl!(DC, layout: Layout::Virtual);
    Ok(())
}

#[cfg(hdf5_1_10_0)]
#[test]
fn test_dcpl_set_chunk_opts() -> hdf5::Result<()> {
    assert!(DC::try_new()?.get_chunk_opts()?.is_none());
    let mut b = DCB::new();
    assert!(b.layout(Layout::Contiguous).finish()?.get_chunk_opts()?.is_none());
    assert!(b.layout(Layout::Compact).finish()?.get_chunk_opts()?.is_none());
    #[cfg(hdf5_1_10_0)]
    assert!(b.layout(Layout::Virtual).finish()?.get_chunk_opts()?.is_none());
    b.layout(Layout::Chunked);
    assert_eq!(b.finish()?.get_chunk_opts()?, Some(ChunkOpts::empty()));
    b.chunk_opts(ChunkOpts::empty());
    assert_eq!(b.finish()?.get_chunk_opts()?, Some(ChunkOpts::empty()));
    b.chunk_opts(ChunkOpts::DONT_FILTER_PARTIAL_CHUNKS);
    assert_eq!(b.finish()?.get_chunk_opts()?, Some(ChunkOpts::DONT_FILTER_PARTIAL_CHUNKS));
    Ok(())
}

#[test]
fn test_dcpl_set_alloc_time() -> hdf5::Result<()> {
    check_matches!(DC::try_new()?.get_alloc_time()?, (), AllocTime::Late);
    let mut b = DCB::new();
    b.alloc_time(None);
    b.layout(Layout::Contiguous);
    check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Late);
    b.layout(Layout::Compact);
    check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Early);
    b.layout(Layout::Chunked);
    check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Incr);
    #[cfg(hdf5_1_10_0)]
    {
        b.layout(Layout::Virtual);
        check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Incr);
    }
    b.layout(Layout::Contiguous);
    b.alloc_time(Some(AllocTime::Late));
    check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Late);
    b.alloc_time(Some(AllocTime::Incr));
    check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Incr);
    b.alloc_time(Some(AllocTime::Early));
    check_matches!(b.finish()?.get_alloc_time()?, (), AllocTime::Early);
    Ok(())
}

#[test]
fn test_dcpl_fill_time() -> hdf5::Result<()> {
    check_matches!(DC::try_new()?.get_fill_time()?, (), FillTime::IfSet);
    check_matches!(DC::try_new()?.fill_time(), (), FillTime::IfSet);
    test_pl!(DC, fill_time: FillTime::IfSet);
    test_pl!(DC, fill_time: FillTime::Alloc);
    test_pl!(DC, fill_time: FillTime::Never);
    Ok(())
}

#[test]
fn test_dcpl_fill_value() -> hdf5::Result<()> {
    use hdf5_derive::H5Type;
    use hdf5_types::{FixedAscii, FixedUnicode, VarLenArray, VarLenAscii, VarLenUnicode};

    check_matches!(DC::try_new()?.get_fill_value_defined()?, (), FillValue::Default);
    check_matches!(DC::try_new()?.fill_value_defined(), (), FillValue::Default);
    assert_eq!(DC::try_new()?.get_fill_value_as::<f64>()?, Some(0.0));
    assert_eq!(DC::try_new()?.fill_value_as::<bool>(), Some(false));

    let mut b = DCB::new();
    b.fill_value(1.23);
    let pl = b.finish()?;
    assert_eq!(pl.fill_value_defined(), FillValue::UserDefined);
    assert_eq!(pl.fill_value_as::<f64>(), Some(1.23));
    assert_eq!(pl.fill_value_as::<i16>(), Some(1));
    assert!(pl.get_fill_value_as::<bool>().is_err());

    #[derive(H5Type, Clone, Debug, PartialEq, Eq)]
    #[repr(C)]
    struct Data {
        a: FixedAscii<5>,
        b: FixedUnicode<5>,
        c: [i16; 2],
        d: VarLenAscii,
        e: VarLenUnicode,
        f: VarLenArray<bool>,
    }

    let data = Data {
        a: FixedAscii::from_ascii(b"12345").unwrap(),
        b: FixedUnicode::from_str("abcd").unwrap(),
        c: [123i16, -1i16],
        d: VarLenAscii::from_ascii(b"xy").unwrap(),
        e: VarLenUnicode::from_str("pqrst").unwrap(),
        f: VarLenArray::from_slice([true, false].as_ref()),
    };
    b.fill_value(data.clone());
    assert_eq!(b.finish()?.fill_value_defined(), FillValue::UserDefined);
    assert_eq!(b.finish()?.fill_value_as::<Data>(), Some(data));
    assert!(b.finish()?.get_fill_value_as::<i16>().is_err());

    Ok(())
}

#[test]
fn test_dcpl_external() -> hdf5::Result<()> {
    assert_eq!(DC::try_new()?.get_external()?, vec![]);
    let pl = DCB::new()
        .external("bar", 0, 1)
        .external("baz", 34, 100)
        .external("foo", 12, 0)
        .finish()?;
    let expected = vec![
        ExternalFile { name: "bar".to_owned(), offset: 0, size: 1 },
        ExternalFile { name: "baz".to_owned(), offset: 34, size: 100 },
        ExternalFile { name: "foo".to_owned(), offset: 12, size: 0 },
    ];
    assert_eq!(pl.get_external()?, expected);
    assert_eq!(pl.external(), expected);
    assert_eq!(DCB::from_plist(&pl)?.finish()?.get_external()?, expected);
    assert!(DCB::new().external("a", 1, 0).external("b", 1, 2).finish().is_err());
    Ok(())
}

#[cfg(hdf5_1_10_0)]
#[test]
fn test_dcpl_virtual_map() -> hdf5::Result<()> {
    use hdf5::Hyperslab;
    use ndarray::s;

    let pl = DC::try_new()?;
    assert!(pl.get_virtual_map().is_err());
    assert_eq!(pl.virtual_map(), vec![]);

    let pl = DCB::new().layout(Layout::Virtual).finish()?;
    assert_eq!(pl.get_virtual_map()?, vec![]);
    assert_eq!(pl.virtual_map(), vec![]);

    let pl = DCB::new()
        .layout(Layout::Virtual)
        .virtual_map("foo", "bar", (3, 4..), (.., 1..), (10..=20, 10), (..3, 7..))
        .virtual_map("x", "y", 100, 91.., 12, Hyperslab::new(s![2..;3]).set_block(0)?)
        .finish()?;
    let expected = vec![
        VirtualMapping {
            src_filename: "foo".into(),
            src_dataset: "bar".into(),
            src_extents: (3, 4..).into(),
            src_selection: (..3, 1..4).into(),
            vds_extents: (10..=20, 10).into(),
            vds_selection: (..3, 7..10).into(),
        },
        VirtualMapping {
            src_filename: "x".into(),
            src_dataset: "y".into(),
            src_extents: 100.into(),
            src_selection: (91..100).into(),
            vds_extents: 12.into(),
            vds_selection: Hyperslab::new(s![2..11;3]).set_block(0)?.into(),
        },
    ];
    assert_eq!(pl.get_virtual_map()?, expected);
    assert_eq!(pl.virtual_map(), expected);

    assert_eq!(DCB::from_plist(&pl)?.finish()?.get_virtual_map()?, expected);

    let mut b = DCB::new()
        .virtual_map("foo", "bar", (3, 4..), (.., 1..), (10..=20, 10), (..3, 7..))
        .clone();

    // layout is set to virtual if virtual map is given
    assert_eq!(b.layout(Layout::Contiguous).finish()?.layout(), Layout::Virtual);
    assert_eq!(b.layout(Layout::Compact).finish()?.layout(), Layout::Virtual);
    assert_eq!(b.layout(Layout::Chunked).finish()?.layout(), Layout::Virtual);

    // chunks are ignored in virtual mode
    assert_eq!(b.chunk((1, 2, 3, 4)).finish()?.layout(), Layout::Virtual);
    assert_eq!(b.chunk((1, 2, 3, 4)).finish()?.chunk(), None);

    Ok(())
}

#[test]
fn test_dcpl_obj_track_times() -> hdf5::Result<()> {
    assert_eq!(DC::try_new()?.get_obj_track_times()?, true);
    assert_eq!(DC::try_new()?.obj_track_times(), true);
    test_pl!(DC, obj_track_times: true);
    test_pl!(DC, obj_track_times: false);
    Ok(())
}

#[test]
fn test_dcpl_attr_phase_change() -> hdf5::Result<()> {
    assert_eq!(DC::try_new()?.get_attr_phase_change()?, AttrPhaseChange::default());
    assert_eq!(DC::try_new()?.attr_phase_change(), AttrPhaseChange::default());
    let pl = DCB::new().attr_phase_change(34, 21).finish()?;
    let expected = AttrPhaseChange { max_compact: 34, min_dense: 21 };
    assert_eq!(pl.get_attr_phase_change()?, expected);
    assert_eq!(pl.attr_phase_change(), expected);
    assert_eq!(DCB::from_plist(&pl)?.finish()?.get_attr_phase_change()?, expected);
    assert!(DCB::new().attr_phase_change(12, 34).finish().is_err());
    Ok(())
}

#[test]
fn test_dcpl_attr_creation_order() -> hdf5::Result<()> {
    assert_eq!(DC::try_new()?.get_attr_creation_order()?.bits(), 0);
    assert_eq!(DC::try_new()?.attr_creation_order().bits(), 0);
    test_pl!(DC, attr_creation_order: AttrCreationOrder::TRACKED);
    test_pl!(DC, attr_creation_order: AttrCreationOrder::TRACKED | AttrCreationOrder::INDEXED);
    assert!(DCB::new().attr_creation_order(AttrCreationOrder::INDEXED).finish().is_err());
    Ok(())
}

type LC = LinkCreate;
type LCB = LinkCreateBuilder;

#[test]
fn test_lcpl_common() -> hdf5::Result<()> {
    test_pl_common!(LC, PropertyListClass::LinkCreate, |b: &mut LCB| b
        .create_intermediate_group(true)
        .finish());
    Ok(())
}

#[test]
fn test_lcpl_create_intermediate_group() -> hdf5::Result<()> {
    assert_eq!(LC::try_new()?.get_create_intermediate_group()?, false);
    assert_eq!(
        LCB::new().create_intermediate_group(false).finish()?.get_create_intermediate_group()?,
        false
    );
    assert_eq!(
        LCB::new().create_intermediate_group(false).finish()?.create_intermediate_group(),
        false
    );
    assert_eq!(
        LCB::new().create_intermediate_group(true).finish()?.get_create_intermediate_group()?,
        true
    );
    assert_eq!(
        LCB::new().create_intermediate_group(true).finish()?.create_intermediate_group(),
        true
    );
    let pl = LCB::new().create_intermediate_group(true).finish()?;
    assert_eq!(LCB::from_plist(&pl)?.finish()?.get_create_intermediate_group()?, true);
    Ok(())
}

#[test]
fn test_lcpl_char_encoding() -> hdf5::Result<()> {
    use hdf5::plist::link_create::CharEncoding;
    assert_eq!(LC::try_new()?.get_char_encoding()?, CharEncoding::Ascii);
    assert_eq!(
        LCB::new().char_encoding(CharEncoding::Ascii).finish()?.get_char_encoding()?,
        CharEncoding::Ascii
    );
    assert_eq!(
        LCB::new().char_encoding(CharEncoding::Ascii).finish()?.char_encoding(),
        CharEncoding::Ascii
    );
    assert_eq!(
        LCB::new().char_encoding(CharEncoding::Utf8).finish()?.get_char_encoding()?,
        CharEncoding::Utf8
    );
    assert_eq!(
        LCB::new().char_encoding(CharEncoding::Utf8).finish()?.char_encoding(),
        CharEncoding::Utf8
    );
    let pl = LCB::new().char_encoding(CharEncoding::Utf8).finish()?;
    assert_eq!(LCB::from_plist(&pl)?.finish()?.get_char_encoding()?, CharEncoding::Utf8);
    Ok(())
}
