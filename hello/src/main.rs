use std::io;
use rand::Rng;

fn main() 
{
    println!("Guess the number!");
    let secret_number = rand::thread_rng().gen_range(1..=100);

    loop {
        println!("Please input your guess :");
        let mut guess = String::new();
        io::stdin()
            .read_line(&mut guess)
            .expect("Failed to read input");

        //let guess : u32 = guess.trim().parse().expect("Please type a number!");
        let guess : u32 = match guess.trim().parse() {
            Ok(num) => num,
            Err(_) => continue
        };

        println!("Your guess : {guess}");

        match guess.cmp( &secret_number ) {
            std::cmp::Ordering::Less =>    println!("Too small"),
            std::cmp::Ordering::Greater => println!("Too big"),
            std::cmp::Ordering::Equal =>   {
                println!("Bulls eye!");
                break;
            }
        }
    }
}
