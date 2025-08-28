fn main() 
{
    println!("Hello, world!");
    let args : Vec<String> = env::args().collect();
    dbg!(&args);
    let tuple : (i32,f32) = (1,1.0);
    
    let mut tuple_vect : Vec<(i32,f32)> = Vec::new();
    tuple_vect.push(tuple);
    let _vect  = vec![1,2,3];

    for arg in &args {
        println!("args {arg}");
    }
    for arg in args {
        println!("args {arg}");
    }
    let mut vec = Vec::new();
    vec.push("dupa");

    let s = String::from("moja dupa");
    for c in s.as_bytes().iter() {
        println!("{c}");
    }
    for c in s.chars() {
        println!("{c}");
    }
    
}

