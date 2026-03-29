use std::env;
use std::fmt;

use braille::{BrailleCharUnOrdered, BrailleCharGridVector};

use glam::Vec3;
use image::{imageops::FilterType, GenericImageView};

struct Buffer {
    pub width: usize,
    pub height: usize,
    pub data: Vec<Vec3>,
}

impl Buffer {
    pub fn new(width: usize, height: usize) -> Self {
        return Self {
            width,
            height,
            data: vec![Vec3::ZERO; width * height]
        };
    }

    pub fn dimensions(&self) -> (usize, usize) {
        return (self.width, self.height);
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize) -> Vec3 {
        return self.data[y * self.width + x];
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, value: Vec3) {
        self.data[y * self.width + x] = value;
    }

    #[inline]
    fn get_mut(&mut self, x: usize, y: usize) -> &mut Vec3 {
        return &mut self.data[y * self.width + x];
    }

    pub fn from_file(img: image::DynamicImage, w: u32, h: u32) -> Self {
        let img = img.resize_exact(w, h, FilterType::Triangle).to_rgb8();
        let (w, h) = img.dimensions();

        let mut buf = Self::new(w as usize, h as usize);

        for y in 0..h as usize {
            for x in 0..w as usize {
                let p = img.get_pixel(x as u32, y as u32).0;
                buf.set(x, y, Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32));
            }
        }

        return buf;
    }

    pub fn dither_pushback(&mut self, x: usize, y: usize, error: Vec3, dithering: &(&[((isize, usize), f32)], f32)) {
        let error = error * dithering.1;

        for ((x_, y_), k) in dithering.0.iter() {
            let x = (x as isize + x_) as usize;
            if x < self.width {
                let y = y + y_;
                if y < self.height {
                    *self.get_mut(x, y) += error * k;
                }
            }
        }
    }
}

const FLOYDSTEINBERG_DITHERING: (&[((isize, usize), f32)], f32) = (&[
    ((1, 0), 7.0),
    ((-1, 1), 3.0),
    ((0, 1), 5.0),
    ((1, 1), 1.0)
], 1.0 / 16.0);
const ATKINSON_DITHERING: (&[((isize, usize), f32)], f32) = (&[
    ((1, 0), 1.0),
    ((2, 0), 1.0),
    ((-1, 1), 1.0),
    ((0, 1), 1.0),
    ((1, 1), 1.0),
    ((0, 2), 1.0)
], 1.0 / 8.0);
const SIERRA1_DITHERING: (&[((isize, usize), f32)], f32) = (&[
    ((1, 0), 5.0),
    ((2, 0), 3.0),
    ((-2, 1), 2.0),
    ((-1, 1), 4.0),
    ((0, 1), 5.0),
    ((1, 1), 4.0),
    ((2, 1), 2.0),
    ((-1, 2), 2.0),
    ((0, 2), 3.0),
    ((1, 2), 2.0),
], 1.0 / 32.0);
const SIERRA2_DITHERING: (&[((isize, usize), f32)], f32) = (&[
    ((1, 0), 4.0),
    ((2, 0), 3.0),
    ((-2, 1), 1.0),
    ((-1, 1), 2.0),
    ((0, 1), 3.0),
    ((1, 1), 2.0),
    ((2, 1), 1.0)
], 1.0 / 16.0);
const SIERRA3_DITHERING: (&[((isize, usize), f32)], f32) = (&[
    ((1, 1), 2.0),
    ((-1, 1), 1.0),
    ((0, 1), 1.0)
], 1.0 / 4.0);
const STUCKI_DITHERING: (&[((isize, usize), f32)], f32) = (&[
    ((1, 0), 8.0),
    ((2, 0), 4.0),
    ((-2, 1), 2.0),
    ((-1, 1), 4.0),
    ((0, 1), 8.0),
    ((1, 1), 4.0),
    ((2, 1), 2.0),
    ((-2, 2), 1.0),
    ((-1, 2), 2.0),
    ((0, 2), 4.0),
    ((1, 2), 2.0),
    ((2, 2), 1.0),
], 1.0 / 42.0);

#[derive(Debug)]
enum Dithering {
    FloydSteinberg,
    Atkinson,
    Sierra1,
    Sierra2,
    Sierra3,
    Stucki
}

impl fmt::Display for Dithering {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let name = match self {
            Self::FloydSteinberg => "Floyd-Steinberg",
            Self::Atkinson => "Atkinson",
            Self::Sierra1 => "Sierra",
            Self::Sierra2 => "Two-Row Sierra",
            Self::Sierra3 => "Sierra Lite",
            Self::Stucki => "Stucki"
        };

        return write!(formatter, "{}", name);
    }
}

fn dither_img(mut img: Buffer, dithering: Dithering) {
    let (w, h) = img.dimensions();

    let mut array: BrailleCharGridVector<BrailleCharUnOrdered> = BrailleCharGridVector::new(w/2, h/4);

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get(x, y);
            let l = pixel.element_sum() /  3.0;
            img.set(x, y, Vec3::splat(l));
        }
    }

    for y in 0..(h/4*4) {
        for x in 0..(w/2*2) {
            let oldpixel = img.get(x, y).clamp(Vec3::ZERO, Vec3::splat(255.0));

            let (b, nl) = match oldpixel.x {
                0.0..127.0 => (false, 0.0),
                _ => (true, 255.0)
            };
            let newpixel = Vec3::splat(nl);

            array.set(x, y, b);

            let quant_error = oldpixel - newpixel;

            let dither = match dithering {
                Dithering::FloydSteinberg => FLOYDSTEINBERG_DITHERING,
                Dithering::Atkinson => ATKINSON_DITHERING,
                Dithering::Sierra1 => SIERRA1_DITHERING,
                Dithering::Sierra2 => SIERRA2_DITHERING,
                Dithering::Sierra3 => SIERRA3_DITHERING,
                Dithering::Stucki => STUCKI_DITHERING
            };

            img.dither_pushback(x, y, quant_error, &dither);
        }
    }

    for y in 0..(h/4) {
        for x in 0..(w/2) {
            print!("{}", array.get_char_unchecked(x, y).char());
        }
        println!();
    }
}

fn main() {
    let mut args = env::args().skip(1);

    let file = args.next().expect("Missing file name");

    let h = file.to_lowercase();
    if h == "-h" || h == "--help" {
        println!("Preview any image file");
        println!("Usage: preview [FILENAME] [OPTIONS] [SETTINGS]");
        println!("Options:");
        println!("    -h, --help                        Print help");
        println!("    -v, --verbose                     Use verbose output: display informations about the resizing");
        println!("Settings:");
        println!("    [int]x[int]                       Specify output dimensions");
        println!("    [int]                             Specify output width: the height gets scaled accordingly");
        println!("    [dithering algorithm]             Specify the dithering algorithm used:");
        println!("        'f' | 'floydsteinberg'            Floyd-Steinberg");
        println!("        'a' | 'atkinson'                  Atkinson -- the default");
        println!("        's' | 's1' | 'sierra | 'sierra1'  Sierra");
        println!("        's2' | 'tworowsierra' | 'sierra2' Two-Row Sierra");
        println!("        's3' | 'sierralite' | 'sierra3'   Sierra Lite");
        println!("        'stucki'                          Stucki");

        return;
    }

    let mut dithering = Dithering::Atkinson;
    let mut width = 0;
    let mut height = 0;

    let mut verbose = false;

    for arg in args {
        let arg = arg.to_lowercase();

        if let Some((a, b)) = arg.split_once('x') {
            match (a.parse::<u32>(), b.parse::<u32>()) {
                (Ok(w), Ok(h)) => {
                    width = w;
                    height = h;
                }
                _ => eprintln!("Error: '{}' is not valid [int]x[int] format", arg),
            }
        }
        else if let Ok(w) = arg.parse::<u32>() {
            width = w;
        }
        else if arg == "-v" || arg == "--verbose" {
            verbose = true;
        }
        else if arg == "floydsteinberg"  || arg == "f" {
            dithering = Dithering::FloydSteinberg;
        }
        else if arg == "atkinson"  || arg == "a" {
            dithering = Dithering::Atkinson;
        }
        else if arg == "s" || arg == "s1" || arg == "sierra" || arg == "sierra1" {
            dithering = Dithering::Sierra1;
        }
        else if arg == "s2" || arg == "tworowsierra" || arg == "sierra2" {
            dithering = Dithering::Sierra2;
        }
        else if arg == "s3" || arg == "sierralite" || arg == "sierra3" {
            dithering = Dithering::Sierra3;
        }
        else if arg == "stucki" {
            dithering = Dithering::Stucki;
        }

        else {
            eprintln!("Error: '{}' is invalid", arg);
        }
    }

    let img = image::open(&file).expect("Missing file name");
    let (w, h) = img.dimensions();

    if height == 0 {
        if width == 0 {
            width = 128;
        }
        height = h * width / w;
    }

    let buffer = Buffer::from_file(img, width, height);

    if verbose {
        println!("Resizing image from {} x {} to {} x {} and dithering using the {} algorithm", w, h, width, height, dithering);
    }

    dither_img(buffer, dithering);
}

