#![allow(dead_code)]

#[cfg(target_arch = "x86")]
#[crochet::load("libavs-win32.dll")]
extern "C" {
    #[symbol("XCd229cc00013c")]
    pub fn property_destroy(property: *mut ()) -> i32;
    #[symbol("XCd229cc000035")]
    pub fn property_set_flag(property: *mut (), set_flags: u32, clear_flags: u32) -> u32;
    #[symbol("XCd229cc00014b")]
    pub fn property_clear_error(property: *mut ()) -> *mut ();
    #[symbol("XCd229cc000032")]
    pub fn property_query_size(property: *const ()) -> i32;
    #[symbol("XCd229cc00012e")]
    pub fn property_search(property: *const (), node: *const (), path: *const u8) -> *mut ();
    #[symbol("XCd229cc000049")]
    pub fn property_node_name(node: *const (), buffer: *mut u8, size: u32) -> i32;
    #[symbol("XCd229cc0000f3")]
    pub fn property_node_read(
        node: *const (),
        node_type: NodeType,
        data: *mut (),
        size: u32,
    ) -> i32;
    #[symbol("XCd229cc000009")]
    pub fn property_node_refer(
        property: *const (),
        node: *const (),
        path: *const u8,
        node_type: NodeType,
        data: *mut (),
        size: u32,
    ) -> i32;
    #[symbol("XCd229cc000033")]
    pub fn property_mem_write(property: *mut (), data: *mut u8, size: u32) -> i32;
}

#[cfg(target_arch = "x86_64")]
#[crochet::load("libavs-win64.dll")]
extern "C" {
    #[symbol("XCnbrep7000091")]
    pub fn property_destroy(property: *mut ()) -> i32;
    #[symbol("XCnbrep700009a")]
    pub fn property_set_flag(property: *mut (), set_flags: u32, clear_flags: u32) -> u32;
    #[symbol("XCnbrep700009d")]
    pub fn property_clear_error(property: *mut ()) -> *mut ();
    #[symbol("XCnbrep700009f")]
    pub fn property_query_size(property: *const ()) -> i32;
    #[symbol("XCnbrep70000a1")]
    pub fn property_search(property: *const (), node: *const (), path: *const u8) -> *mut ();
    #[symbol("XCnbrep70000a7")]
    pub fn property_node_name(node: *const (), buffer: *mut u8, size: u32) -> i32;
    #[symbol("XCnbrep70000ab")]
    pub fn property_node_read(
        node: *const (),
        node_type: NodeType,
        data: *mut (),
        size: u32,
    ) -> i32;
    #[symbol("XCnbrep70000af")]
    pub fn property_node_refer(
        property: *const (),
        node: *const (),
        path: *const u8,
        node_type: NodeType,
        data: *mut (),
        size: u32,
    ) -> i32;
    #[symbol("XCnbrep70000b8")]
    pub fn property_mem_write(property: *mut (), data: *mut u8, size: u32) -> i32;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum NodeType {
    NodeNode = 1,
    NodeS8 = 2,
    NodeU8 = 3,
    NodeS16 = 4,
    NodeU16 = 5,
    NodeS32 = 6,
    NodeU32 = 7,
    NodeS64 = 8,
    NodeU64 = 9,
    NodeBin = 10,
    NodeStr = 11,
    NodeIp4 = 12,
    NodeTime = 13,
    NodeFloat = 14,
    NodeDouble = 15,
    Node2s8 = 16,
    Node2u8 = 17,
    Node2s16 = 18,
    Node2u16 = 19,
    Node2s32 = 20,
    Node2u32 = 21,
    Node2s64 = 22,
    Node2u64 = 23,
    Node2f = 24,
    Node2d = 25,
    Node3s8 = 26,
    Node3u8 = 27,
    Node3s16 = 28,
    Node3u16 = 29,
    Node3s32 = 30,
    Node3u32 = 31,
    Node3s64 = 32,
    Node3u64 = 33,
    Node3f = 34,
    Node3d = 35,
    Node4s8 = 36,
    Node4u8 = 37,
    Node4s16 = 38,
    Node4u16 = 39,
    Node4s32 = 40,
    Node4u32 = 41,
    Node4s64 = 42,
    Node4u64 = 43,
    Node4f = 44,
    Node4d = 45,
    NodeAttr = 46,
    NodeAttrAndNode = 47,
    NodeVs8 = 48,
    NodeVu8 = 49,
    NodeVs16 = 50,
    NodeVu16 = 51,
    NodeBool = 52,
    Node2b = 53,
    Node3b = 54,
    Node4b = 55,
    NodeVb = 56,
}
