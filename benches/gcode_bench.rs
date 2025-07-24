// Benchmark for G-code parser and macro expansion performance
// Run with: cargo bench

use criterion::{criterion_group, criterion_main, Criterion};
use krusty_rs::gcode::parser::{GCodeParser, GCodeParserConfig};
use krusty_rs::gcode::macros::MacroProcessor;

fn bench_gcode_parser(c: &mut Criterion) {
    let mut gcode = String::new();
    for i in 0..10_000 {
        gcode.push_str(&format!("G1 X{} Y{} F1500\n", i, i));
    }
    c.bench_function("parse 10k G1 lines", |b| {
        b.iter(|| {
            let mut parser = GCodeParser::new(&gcode, GCodeParserConfig::default());
            let mut count = 0;
            while let Some(cmd) = parser.next_command() {
                if cmd.is_ok() { count += 1; }
            }
            assert_eq!(count, 10_000);
        });
    });
}

fn bench_macro_expansion(c: &mut Criterion) {
    let macro_processor = MacroProcessor::new();
    let macro_body = (0..1000).map(|i| format!("G1 X{} Y{} F1500", i, i)).collect::<Vec<_>>();
    let rt = tokio::runtime::Runtime::new().unwrap();
    // Define the macro only once before benchmarking
    rt.block_on(async {
        macro_processor.define_macro("BIGMACRO", macro_body).await.unwrap();
    });
    c.bench_function("expand BIGMACRO (1000 lines)", |b| {
        b.iter(|| {
            rt.block_on(async {
                let expanded = macro_processor.parse_and_expand_async_owned("{BIGMACRO}").await;
                let ok_count = expanded.iter().filter(|r| r.is_ok()).count();
                assert_eq!(ok_count, 1000);
            });
        });
    });
}

criterion_group!(benches, bench_gcode_parser, bench_macro_expansion);
criterion_main!(benches);
