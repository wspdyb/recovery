import struct
import os

def png_to_ico(png_path, ico_path):
    if not os.path.exists(png_path):
        print(f"Error: {png_path} not found")
        return

    with open(png_path, 'rb') as f:
        png_data = f.read()

    # ICO Header: Reserved(2), Type(2), Count(2)
    header = struct.pack('<HHH', 0, 1, 1)

    # Image Entry: Width(1), Height(1), Colors(1), Reserved(1), Planes(2), BitCount(2), Size(4), Offset(4)
    # Using 0 for width/height signifies 256px
    entry = struct.pack('<BBBBHHII', 0, 0, 0, 0, 1, 32, len(png_data), 6 + 16)

    with open(ico_path, 'wb') as f:
        f.write(header)
        f.write(entry)
        f.write(png_data)
    print(f"Successfully created {ico_path}")

png_to_ico('apps/desktop/src-tauri/icons/icon.png', 'apps/desktop/src-tauri/icons/icon.ico')
