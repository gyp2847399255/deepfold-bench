use hyrax::bench;

fn main() {
    for i in 10..23 {
        bench(i, 10);
    }
}
