import struct

def create_blank_ico(ico_path):
    width = 32
    height = 32
    # Icon Header
    header = struct.pack('<HHH', 0, 1, 1)

    # Bitmap Info Header (40 bytes) + Pixels (4096 bytes) + Mask (128 bytes)
    image_data_size = 40 + (width * height * 4) + (width * height // 8)

    # Directory Entry
    entry = struct.pack('<BBBBHHII', width, height, 0, 0, 1, 32, image_data_size, 6 + 16)

    # Bitmap Info Header (biHeight is doubled for XOR+AND masks)
    bi_header = struct.pack('<IiiHHIIiiII', 40, width, height * 2, 1, 32, 0, width * height * 4, 0, 0, 0, 0)

    # Pixels (All blue for testing)
    pixels = b'\xFF\x00\x00\xFF' * (width * height) # BGRA (Blue)

    # Mask (All zeros = no transparency)
    mask = b'\x00' * (width * height // 8)

    with open(ico_path, 'wb') as f:
        f.write(header)
        f.write(entry)
        f.write(bi_header)
        f.write(pixels)
        f.write(mask)
    print(f"Created standard 32x32 32bpp ICO: {ico_path}")

create_blank_ico('apps/desktop/src-tauri/icons/icon.ico')
