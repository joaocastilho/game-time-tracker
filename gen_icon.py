import sys
from PIL import Image

def create_ico():
    bg_color = (40, 44, 52, 255)
    fg_color = (97, 175, 239, 255)
    accent_color = (152, 195, 121, 255)
    
    img = Image.new('RGBA', (32, 32), (0, 0, 0, 0))
    pixels = img.load()
    
    for y in range(32):
        for x in range(32):
            cx = x - 16
            cy = y - 16
            dist = (cx * cx + cy * cy) ** 0.5
            
            if dist < 14.0:
                pixels[x, y] = bg_color
                
                is_center = abs(cx) < 3 and abs(cy) < 6
                is_left_btn = abs(cx + 6) < 2 and abs(cy - 3) < 2
                is_right_btn = abs(cx - 6) < 2 and abs(cy - 3) < 2
                is_left_analog = abs(cx + 5) < 3 and abs(cy + 3) < 3
                is_right_analog = abs(cx - 5) < 3 and abs(cy + 3) < 3
                
                if is_center or is_left_btn or is_right_btn:
                    pixels[x, y] = fg_color
                elif is_left_analog or is_right_analog:
                    pixels[x, y] = accent_color
                    
    img.save('assets/icon.ico', format='ICO')
    print("Saved assets/icon.ico")

if __name__ == "__main__":
    create_ico()
