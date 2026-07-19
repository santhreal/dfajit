use dfajit::JitDfa;
use matchkit::Match;

fn main() {
    let jit = JitDfa::from_regex_patterns(&["a+"]).unwrap();
    let mut matches = vec![Match::from_parts(0, 0, 0); 20];
    let count = jit.scan(b"aaab", &mut matches);
    println!("count = {}", count);
    for (i, m) in matches.iter().enumerate().take(count.min(matches.len())) {
        println!(
            "match {i}: pid={} start={} end={}",
            m.pattern_id, m.start, m.end
        );
    }
}
