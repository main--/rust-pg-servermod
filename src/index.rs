use std::os::raw::c_void;
use std::ptr;
use types::Oid;

// prime example for extern types, oh well
type Relation = *mut c_void;
type IndexScanDesc = *mut c_void;
extern "C" {
    fn heap_open(relation: Oid, lockmode: i32) -> Relation;
    fn relation_close(relation: Relation, lockmode: i32);
    fn index_open(relation: Oid, lockmode: i32 /* set to 1 */) -> Relation; // open index relation
    fn index_close(relation: Relation, lockmode: i32);
    fn index_beginscan(heap: Relation, index: Relation, snapshot: *mut c_void /* null */, nkeys: i32, norderbys: i32) -> IndexScanDesc;
    fn index_rescan(scan: IndexScanDesc, scankeys: *mut u8, nkeys: i32, orderbys: *mut u8, norderbys: i32);
    fn index_getnext(scan: IndexScanDesc, direction: i32) -> *mut c_void; // returns HeapTuple
    //fn index_getnext_tid(scan: IndexScanDesc, direction: i32) -> *mut c_void; // returns ItemPointer
    fn index_endscan(scan: IndexScanDesc);
    fn GetTransactionSnapshot() -> *mut c_void;
    fn ScanKeyInit(entry: *mut u8, attr_num: u16, strat_num: u16, regproc: u32, arg: usize);
}

pub fn do_index_scan(rel: Oid, idx: Oid) -> i32 {
    let mut counter = 0;
    unsafe {
        let heap = heap_open(rel, 1);
        let index = index_open(idx, 1);

        let btint4cmp = 184;
        let mut keybuf = [0u8; ::LEN_SCANKEYDATA];
        ScanKeyInit(keybuf.as_mut_ptr(), 1, 3, btint4cmp, 4);

        let snap = GetTransactionSnapshot();
        assert!(!snap.is_null());
        let scan = index_beginscan(heap, index, snap, 1, 0);
        index_rescan(scan, keybuf.as_mut_ptr(), 1, ptr::null_mut(), 0);
        loop {
            let thing = index_getnext(scan, 1);
            if thing.is_null() { break; }
            counter += 1;
        }
        index_endscan(scan);

        index_close(index, 1);
        relation_close(heap, 1);
    }
    counter
}
