import struct
import os
import json

def create_mft_record(mft_id, filename, is_deleted=False):
    record = bytearray(1024)
    record[0:4] = b"FILE"
    struct.pack_into("<H", record, 4, 48) # Update sequence offset
    struct.pack_into("<H", record, 6, 2) # Update sequence size
    struct.pack_into("<Q", record, 8, 1) # LSN
    struct.pack_into("<H", record, 16, 1) # Link count
    struct.pack_into("<H", record, 20, 56) # Attribute offset

    flags = 0x00 if is_deleted else 0x01
    struct.pack_into("<H", record, 22, flags)

    struct.pack_into("<I", record, 24, 1024) # Used size
    struct.pack_into("<I", record, 28, 1024) # Allocated size

    # Add $FILENAME attribute
    attr_off = 56
    filename_bin = filename.encode("utf-16le")
    fn_len_chars = len(filename)
    fn_len_bytes = len(filename_bin)

    content_size = 66 + fn_len_bytes
    attr_size = 16 + 8 + content_size

    struct.pack_into("<I", record, attr_off, 0x30)
    struct.pack_into("<I", record, attr_off + 4, attr_size)
    record[attr_off + 8] = 0 # Resident
    struct.pack_into("<H", record, attr_off + 10, 24) # Content offset
    struct.pack_into("<I", record, attr_off + 16, content_size) # Content size

    content_off = attr_off + 24
    struct.pack_into("<Q", record, content_off, 5) # Parent
    record[content_off + 64] = fn_len_chars
    record[content_off + 65] = 0x01 # Win32
    record[content_off + 66 : content_off + 66 + fn_len_bytes] = filename_bin

    return record

def create_vbr(mft_lcn=4, magic=b"NTFS    "):
    vbr = bytearray(512)
    vbr[3:3+len(magic)] = magic
    struct.pack_into("<H", vbr, 0x0B, 512) # Bytes per sector
    vbr[0x0D] = 8 # Sectors per cluster
    struct.pack_into("<Q", vbr, 0x30, mft_lcn) # MFT LCN
    return vbr

def save_test(dir_path, name, data, expected):
    full_dir = os.path.join("testdata", "ntfs", dir_path)
    os.makedirs(full_dir, exist_ok=True)
    with open(os.path.join(full_dir, f"{name}.img"), "wb") as f:
        f.write(data)
    with open(os.path.join(full_dir, "expected.json"), "w") as f:
        json.dump(expected, f, indent=2)
    print(f"Generated {dir_path}")

def generate_corpus():
    # 1. tiny_valid
    data = bytearray(1024 * 1024)
    data[0:512] = create_vbr()
    mft_base = 4 * 8 * 512
    data[mft_base : mft_base + 1024] = create_mft_record(0, "MFT_Valid.bin")
    data[mft_base + 1024 : mft_base + 2048] = create_mft_record(1, "Document.docx")
    save_test("tiny_valid", "valid", data, {
        "expect_ntfs": True,
        "min_artifacts": 2,
        "expected_names": ["MFT_Valid.bin", "Document.docx"],
        "has_deleted": False
    })

    # 2. tiny_deleted
    data = bytearray(1024 * 1024)
    data[0:512] = create_vbr()
    data[mft_base : mft_base + 1024] = create_mft_record(0, "ActiveFile.txt", is_deleted=False)
    data[mft_base + 1024 : mft_base + 2048] = create_mft_record(1, "Deleted1.bak", is_deleted=True)
    data[mft_base + 2048 : mft_base + 3072] = create_mft_record(2, "OldData.old", is_deleted=True)
    save_test("tiny_deleted", "deleted", data, {
        "expect_ntfs": True,
        "min_artifacts": 3,
        "expected_names": ["ActiveFile.txt", "Deleted1.bak", "OldData.old"],
        "has_deleted": True
    })

    # 3. whole_disk
    data = bytearray(2 * 1024 * 1024)
    # MBR (Sector 0)
    # Partition 1: Start LBA 2048 (1MB offset), Size 2048
    struct.pack_into("<B", data, 446 + 4, 0x07) # NTFS type
    struct.pack_into("<I", data, 446 + 8, 2048) # Start LBA
    struct.pack_into("<I", data, 446 + 12, 2048) # Size
    data[510] = 0x55
    data[511] = 0xAA

    ntfs_off = 2048 * 512
    data[ntfs_off : ntfs_off + 512] = create_vbr()
    mft_off = ntfs_off + (4 * 8 * 512)
    data[mft_off : mft_off + 1024] = create_mft_record(0, "WholeDiskFile.txt")
    save_test("whole_disk", "disk", data, {
        "expect_ntfs": True,
        "layout": "MBR",
        "ntfs_offset": ntfs_off,
        "expected_names": ["WholeDiskFile.txt"]
    })

    # 4. corrupt_vbr
    data = bytearray(512 * 8)
    data[0:512] = create_vbr(magic=b"BAD_NTFS")
    save_test("corrupt_vbr", "corrupt", data, {
        "expect_ntfs": False,
        "error_type": "InvalidSignature"
    })

    # 5. non_ntfs
    data = bytearray(512 * 8)
    data[3:11] = b"FAT32   "
    save_test("non_ntfs", "random", data, {
        "expect_ntfs": False,
        "error_type": "FileSystemTypeMismatch"
    })

if __name__ == "__main__":
    generate_corpus()
