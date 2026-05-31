import os
from PIL import Image, ImageDraw, ImageFilter, ImageFont

def create_glow(draw, center, radius, color, max_opacity=0.4):
    """Draws a smooth radial glow using concentric circles of decreasing opacity."""
    cx, cy = center
    # Draw concentric circles with step size 1px
    for r in range(radius, 0, -1):
        # Quadratic decay of opacity from max_opacity to 0
        t = (r / radius) ** 2
        opacity = int(255 * max_opacity * (1.0 - t))
        if opacity <= 0:
            continue
        c_fill = (color[0], color[1], color[2], opacity)
        draw.ellipse([cx - r, cy - r, cx + r, cy + r], fill=c_fill)

def get_bracket_polygon(corner, scale=1.0, center=(512, 512)):
    """
    Returns the points for a chamfered corner bracket in a 1024x1024 design space.
    The points are scaled, translated and centered.
    """
    cx_base, cy_base = center
    # Base dimensions for Top-Left corner bracket at scale 1.0 (in 1024x1024 space)
    outer_min = 160
    inner_min = 224
    arm_length = 360
    chamfer_outer = 80
    chamfer_inner = 56
    cut_arm = 64
    
    # Generate Top-Left points
    tl_points = [
        (outer_min, outer_min + arm_length),
        (outer_min, outer_min + chamfer_outer),
        (outer_min + chamfer_outer, outer_min),
        (outer_min + arm_length, outer_min),
        (outer_min + arm_length - cut_arm, inner_min),
        (inner_min + chamfer_inner, inner_min),
        (inner_min, inner_min + chamfer_inner),
        (inner_min, outer_min + arm_length - cut_arm),
    ]
    
    # Transform points based on the corner
    points = []
    for x, y in tl_points:
        # Center of canvas base is (512, 512)
        dx = x - 512
        dy = y - 512
        
        if corner == 'TL':
            px, py = 512 + dx, 512 + dy
        elif corner == 'TR':
            px, py = 512 - dx, 512 + dy
        elif corner == 'BR':
            px, py = 512 - dx, 512 - dy
        elif corner == 'BL':
            px, py = 512 + dx, 512 - dy
            
        # Apply scaling and final center translation
        px = cx_base + (px - 512) * scale
        py = cy_base + (py - 512) * scale
        points.append((px, py))
        
    return points

def draw_logo_mark(draw, center, size, variant='primary', scale_factor=1.0):
    """
    Draws the logo mark onto an existing ImageDraw object at a specific center and size.
    Variants: 'primary', 'white_mono', 'green_mono', 'micro_fav'.
    """
    cx, cy = center
    scale = (size / 1024.0) * scale_factor
    
    if variant == 'primary':
        # Green ambient glow behind brackets
        create_glow(draw, (cx, cy), int(400 * scale), (0, 232, 138), max_opacity=0.08)
        
        # White-silver brackets
        bracket_color = (234, 236, 245, 255) # #eaecf5
        for corner in ['TL', 'TR', 'BR', 'BL']:
            poly = get_bracket_polygon(corner, scale=scale, center=(cx, cy))
            draw.polygon(poly, fill=bracket_color)
            
        # Green dot glow
        create_glow(draw, (cx, cy), int(180 * scale), (0, 232, 138), max_opacity=0.35)
        
        # Green central recording dot
        r_dot = int(60 * scale)
        draw.ellipse([cx - r_dot, cy - r_dot, cx + r_dot, cy + r_dot], fill=(0, 232, 138, 255))
        
    elif variant == 'white_mono':
        white = (255, 255, 255, 255)
        for corner in ['TL', 'TR', 'BR', 'BL']:
            poly = get_bracket_polygon(corner, scale=scale, center=(cx, cy))
            draw.polygon(poly, fill=white)
        r_dot = int(60 * scale)
        draw.ellipse([cx - r_dot, cy - r_dot, cx + r_dot, cy + r_dot], fill=white)
        
    elif variant == 'green_mono':
        green = (0, 232, 138, 255)
        for corner in ['TL', 'TR', 'BR', 'BL']:
            poly = get_bracket_polygon(corner, scale=scale, center=(cx, cy))
            draw.polygon(poly, fill=green)
        r_dot = int(60 * scale)
        draw.ellipse([cx - r_dot, cy - r_dot, cx + r_dot, cy + r_dot], fill=green)
        
    elif variant == 'micro_fav':
        # TL bracket only
        bracket_color = (234, 236, 245, 255)
        poly = get_bracket_polygon('TL', scale=scale, center=(cx, cy))
        draw.polygon(poly, fill=bracket_color)
        
        # Green dot glow
        create_glow(draw, (cx, cy), int(180 * scale), (0, 232, 138), max_opacity=0.45)
        
        # Green central recording dot
        r_dot = int(60 * scale)
        draw.ellipse([cx - r_dot, cy - r_dot, cx + r_dot, cy + r_dot], fill=(0, 232, 138, 255))

def generate_primary_logo(output_path, size=1024):
    """Generates the primary EasySpecy logo and saves it."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw.rectangle([0, 0, size, size], fill=(13, 15, 26, 255)) # Deep space indigo #0d0f1a
    
    draw_logo_mark(draw, (size // 2, size // 2), size, variant='primary')
    img.save(output_path, "PNG")
    print(f"Generated primary logo at {output_path}")

def generate_micro_favicon(output_path, size=512):
    """Generates the micro favicon (TL bracket + dot) and saves it."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_logo_mark(draw, (size // 2, size // 2), size, variant='micro_fav')
    img.save(output_path, "PNG")
    print(f"Generated micro favicon at {output_path}")

def generate_white_monochrome(output_path, size=512):
    """Generates the white monochrome logo variant and saves it."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_logo_mark(draw, (size // 2, size // 2), size, variant='white_mono')
    img.save(output_path, "PNG")
    print(f"Generated white monochrome logo at {output_path}")

def generate_green_monochrome(output_path, size=512):
    """Generates the green monochrome logo variant and saves it."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw_logo_mark(draw, (size // 2, size // 2), size, variant='green_mono')
    img.save(output_path, "PNG")
    print(f"Generated green monochrome logo at {output_path}")

def generate_squircle_tile(output_path, size=512):
    """Generates the squircle tile version (logo centered inside a rounded rect border container)."""
    img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    draw.rectangle([0, 0, size, size], fill=(13, 15, 26, 255))
    
    # Border
    border_color = (42, 45, 66, 255) # #2a2d42
    border_w = int(20 * (size / 1024.0))
    rad = int(160 * (size / 1024.0))
    inset = int(32 * (size / 1024.0))
    draw.rounded_rectangle([inset, inset, size - inset, size - inset], radius=rad, outline=border_color, width=max(1, border_w))
    
    # Draw logo mark centered at 80% scale
    draw_logo_mark(draw, (size // 2, size // 2), size, variant='primary', scale_factor=0.8)
    img.save(output_path, "PNG")
    print(f"Generated squircle tile logo at {output_path}")

def generate_social_previews(output_dir):
    """
    Generates a beautiful tech brand sheet composite banner (1280x640 and 1200x630)
    recreating the design style provided by the user.
    """
    for width, height, filename in [(1280, 640, "github-preview.png"), (1200, 630, "og-image.png"), (1200, 600, "twitter-card.png")]:
        img = Image.new("RGBA", (width, height), (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        
        # Deep space background
        draw.rectangle([0, 0, width, height], fill=(13, 15, 26, 255))
        
        # Render small "01" (Style Indicator) top left
        try:
            mono_font_sm = ImageFont.truetype("arial.ttf", 16)
        except OSError:
            mono_font_sm = ImageFont.load_default()
        draw.text((48, 48), "01", fill=(0, 232, 138, 255), font=mono_font_sm)
        
        # Determine center y
        cy = height // 2
        
        # Left side logo mark
        draw_logo_mark(draw, (160, cy), 160, variant='primary')
        
        # Left side wordmark & tagline
        try:
            font_title = ImageFont.truetype("arial.ttf", 60)
            font_subtitle = ImageFont.truetype("arial.ttf", 22)
        except OSError:
            font_title = ImageFont.load_default()
            font_subtitle = ImageFont.load_default()
            
        # Draw "EasySpecy" title text
        draw.text((270, cy - 45), "EasySpecy", fill=(255, 255, 255, 255), font=font_title)
        
        # Draw Tagline: "Record like a pro. Pay like it's 2005."
        # Split into grey and green segments
        txt_grey = "Record like a pro. "
        txt_green = "Pay like it's 2005."
        
        # In Pillow, we measure text length to draw segments inline
        try:
            grey_w = draw.textlength(txt_grey, font=font_subtitle)
        except AttributeError:
            grey_w = font_subtitle.getmask(txt_grey).getbbox()[2] if font_subtitle.getmask(txt_grey).getbbox() else 180
            
        draw.text((270, cy + 25), txt_grey, fill=(154, 158, 181, 255), font=font_subtitle) # #9a9eb5
        draw.text((270 + grey_w, cy + 25), txt_green, fill=(0, 232, 138, 255), font=font_subtitle) # #00e88a
        
        # Divider Line at x = 640
        divider_x = int(width * 0.50)
        draw.line([divider_x, cy - 100, divider_x, cy + 100], fill=(42, 45, 66, 255), width=2) # #2a2d42
        
        # Right side elements:
        # 1. Squircle container logo
        sq_cx = divider_x + 100
        # Draw squircle container border and background
        border_color = (42, 45, 66, 255)
        # container size is 120x120
        c_left, c_top = sq_cx - 60, cy - 60
        c_right, c_bottom = sq_cx + 60, cy + 60
        draw.rounded_rectangle([c_left, c_top, c_right, c_bottom], radius=24, outline=border_color, width=2)
        draw_logo_mark(draw, (sq_cx, cy), 120, variant='primary', scale_factor=0.8)
        
        # 2. White monochrome outline logo
        wm_cx = sq_cx + 140
        draw_logo_mark(draw, (wm_cx, cy), 100, variant='white_mono')
        
        # 3. Green monochrome outline logo
        gm_cx = wm_cx + 130
        draw_logo_mark(draw, (gm_cx, cy), 100, variant='green_mono')
        
        # 4. Micro Favicon logo
        mf_cx = gm_cx + 120
        draw_logo_mark(draw, (mf_cx, cy), 64, variant='micro_fav')
        
        out_path = os.path.join(output_dir, filename)
        img.save(out_path, "PNG")
        print(f"Generated social preview banner at {out_path}")

def generate_favicons_ico(public_dir):
    """Combines 16, 32, 48 size favicon PNGs into a multi-res ICO file."""
    generate_micro_favicon(os.path.join(public_dir, "favicon-16.png"), 16)
    generate_primary_logo(os.path.join(public_dir, "favicon-32.png"), 32)
    generate_primary_logo(os.path.join(public_dir, "favicon-48.png"), 48)
    generate_primary_logo(os.path.join(public_dir, "favicon-96.png"), 96)
    generate_primary_logo(os.path.join(public_dir, "favicon-128.png"), 128)
    generate_primary_logo(os.path.join(public_dir, "favicon-192.png"), 192)
    
    img16 = Image.open(os.path.join(public_dir, "favicon-16.png"))
    img32 = Image.open(os.path.join(public_dir, "favicon-32.png"))
    img48 = Image.open(os.path.join(public_dir, "favicon-48.png"))
    
    ico_path = os.path.join(public_dir, "favicon.ico")
    img16.save(ico_path, format="ICO", sizes=[(16, 16), (32, 32), (48, 48)], append_images=[img32, img48])
    print(f"Generated multi-resolution favicon.ico at {ico_path}")

def main():
    brand_dir = "c:/Users/shaur/OneDrive/Documents/EasySpecy/brand"
    public_dir = "c:/Users/shaur/OneDrive/Documents/EasySpecy/public"
    
    os.makedirs(brand_dir, exist_ok=True)
    os.makedirs(public_dir, exist_ok=True)
    
    # 1. Generate primary logo in public and brand
    generate_primary_logo(os.path.join(brand_dir, "logo.png"), 1024)
    generate_primary_logo(os.path.join(public_dir, "logo.png"), 512)
    
    # 2. Generate Apple touch icon (180x180) and Android icons
    generate_primary_logo(os.path.join(public_dir, "apple-touch-icon.png"), 180)
    generate_primary_logo(os.path.join(public_dir, "android-chrome-192.png"), 192)
    generate_primary_logo(os.path.join(public_dir, "android-chrome-512.png"), 512)
    
    # 3. Generate all favicons and ICO file
    generate_favicons_ico(public_dir)
    
    # 4. Generate visual variations inside the brand directory
    generate_white_monochrome(os.path.join(brand_dir, "logo_white_mono.png"), 512)
    generate_green_monochrome(os.path.join(brand_dir, "logo_green_mono.png"), 512)
    generate_squircle_tile(os.path.join(brand_dir, "logo_squircle.png"), 512)
    generate_micro_favicon(os.path.join(brand_dir, "logo_micro_fav.png"), 512)
    
    # 5. Generate beautiful social composite banners
    generate_social_previews(public_dir)
    # copy the primary banner to brand directory for developer review
    import shutil
    shutil.copy(os.path.join(public_dir, "github-preview.png"), os.path.join(brand_dir, "github-preview.png"))
    
    print("All assets generated successfully!")

if __name__ == "__main__":
    main()
