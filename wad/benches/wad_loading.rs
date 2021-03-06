use criterion::*;
use wad::wad::WadData;

fn bench_doom1(b: &mut Bencher, _i: &u32) {
    b.iter(|| {
        let mut wad = WadData::new("../doom1.wad");
        wad.read_directories();
    });
}

fn bench_doom(b: &mut Bencher, _i: &u32) {
    b.iter(|| {
        let mut wad = WadData::new("../doom.wad");
        wad.read_directories();
    });
}

fn bench_doom2(b: &mut Bencher, _i: &u32) {
    b.iter(|| {
        let mut wad = WadData::new("../doom2.wad");
        wad.read_directories();
    });
}

fn bench(c: &mut Criterion) {
    let fun = vec![
        Fun::new("Load and read shareware wad", bench_doom1),
        Fun::new("Load and read ultimate wad", bench_doom),
        Fun::new("Load and read Doom II wad", bench_doom2),
    ];
    c.bench_functions("WAD Loading", fun, 100);
}

criterion_group!(benches, bench,);
criterion_main!(benches);
