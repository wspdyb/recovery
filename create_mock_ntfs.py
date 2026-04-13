import struct

def create_mock_ntfs(path):
    size = 1024 * 1024
    data = bytearray(size)

    # VBR
    data[3:11] = b"NTFS    "
    struct.pack_into("<H", data, 0x0B, 512) # Bytes per sector
    data[0x0D] = 8 # Sectors per cluster
    struct.pack_into("<Q", data, 0x30, 4) # MFT Cluster LCN

    # MFT Record 0 (Offset 16384)
    mft_offset = 16384
    record = bytearray(1024)
    record[0:4] = b"FILE"
    struct.pack_into("<H", record, 4, 9) # Update sequence offset
    struct.pack_into("<H", record, 6, 2) # Update sequence size
    struct.pack_into("<Q", record, 8, 0) # LSN
    struct.pack_into("<H", record, 16, 1) # Link count
    struct.pack_into("<H", record, 20, 56) # Attribute offset (was 18)
    struct.pack_into("<H", record, 22, 1) # Flags: In Use
    struct.pack_into("<I", record, 24, 1024) # Used size
    struct.pack_into("<I", record, 28, 1024) # Allocated size

    # Add $FILENAME attribute at offset 56
    attr_off = 56
    attr_type = 0x30
    filename_str = "MFT_Mock.txt"
    filename_bin = filename_str.encode("utf-16le")
    fn_len_chars = len(filename_str)
    fn_len_bytes = len(filename_bin)

    content_size = 66 + fn_len_bytes # minimal size for $FILENAME content
    attr_size = 16 + 8 + content_size # header + content

    # Resident Attribute Header
    struct.pack_into("<I", record, attr_off, attr_type)
    struct.pack_into("<I", record, attr_off + 4, attr_size)
    record[attr_off + 8] = 0 # Resident
    struct.pack_into("<H", record, attr_off + 10, 24) # Content offset relative to attr start
    struct.pack_into("<I", record, attr_off + 16, content_size) # Content size

    # Content starting at attr_off + 24
    content_off = attr_off + 24
    struct.pack_into("<Q", record, content_off, 5) # Parent MFT (Root)
    record[content_off + 64] = fn_len_chars # Filename length in chars
    record[content_off + 65] = 0x01 # Filename type (Win32)
    record[content_off + 66 : content_off + 66 + fn_len_bytes] = filename_bin

    data[mft_offset : mft_offset + 1024] = record

    with open(path, "wb") as f:
        f.write(data)
    print(f"Mock NTFS image created at {path}")

if __name__ == "__main__":
    create_mock_ntfs("test_ntfs.img")
