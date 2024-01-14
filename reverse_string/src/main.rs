use std::collections::VecDeque;
use std::env;
use std::time::Instant;

fn main() {
    println!("Hello, world!");
    let args: Vec<String> = env::args().collect();
    //let args2: Vec<str> = env::args().collect(); error : the trait `Sized` is not implemented for `str`
    //dbg!(args);
    let first = &args[1];

    let st = Instant::now();
    for _ in 0..1000 {
        let _rev = reverse_1(first);
    }
    println!("rev1 duration {:?}",st.elapsed()/1000);

    let st = Instant::now();
    for _ in 0..1000 {
        let _rev = reverse_2(&args[1]);
    }
    println!("rev2 duration {:?}",st.elapsed()/1000);

    let st = Instant::now();
    for _ in 0..1000 {
        let _rev = reverse_3(first.as_str());
    }
    println!("rev3 duration {:?}",st.elapsed()/1000);

    let st = Instant::now();
    println!("rev4 result {}",reverse_4(first));
    for _ in 0..1000 {
        let _rev = reverse_4(first);
    }
    println!("rev4 duration {:?}",st.elapsed()/1000);

    let st = Instant::now();
    for _ in 0..1000 {
        let _rev = reverse_5(first);
    }
    println!("rev5 duration {:?}",st.elapsed()/1000);

    let st = Instant::now();
    println!("rev 6 result {}",reverse_6(first));
    for _ in 0..1000 {
        let _rev = reverse_6(first);
    }
    println!("rev6 duration {:?}",st.elapsed()/1000);
}

fn reverse_1(input: &str) -> String
{
    let mut rev = String::new();
    rev.reserve_exact(input.len());
    for c in input.chars() {
        rev.insert(0,c);
    }
    rev
}
fn reverse_2(input: &str) -> String
{
    let mut chars : Vec<char> = input.chars().collect();
    chars.reverse();
    String::from_iter(chars)
}
fn reverse_3(input: &str) -> String
{
    input.chars().rev().collect()
}

fn reverse_4(input: &str) -> String
{
    let mut chars : Vec<char> = input.chars().collect();
    let last = chars.len()-1;
    for i in 0..chars.len()/2 {
        let c = chars[i];
        chars[i] = chars[last-i];
        chars[last-i]=c;
    }
    String::from_iter(chars)
}

fn reverse_5(input: &str) -> String
{
    let mut rev : VecDeque<char> = VecDeque::new();
    rev.reserve_exact(input.len());
    for c in input.chars() {
        rev.push_front(c);
    }
    String::from_iter(rev)
}

fn reverse_6(input: &str) -> String
{
    let mut rev = String::new();
    rev.reserve_exact(input.len());
    let last = input.len()-1;
    for (i,_c) in input.char_indices() {
        let _pos : i32 = (last - i) as i32;
        //rev[pos] = c; error : String cannot be indexed by `i32`
    }
    rev
}