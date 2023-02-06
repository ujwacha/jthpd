use jthpd::get_addr_thread;
use jthpd::handle_connection;
use jthpd::print_help;
use jthpd::ThreadPool;
use std::net::TcpListener;

fn main() {
    print_help();
    let (adress, threads) = get_addr_thread();
    let socket = TcpListener::bind(adress).unwrap();
    println!("made a socket");
    let pool = ThreadPool::new(threads);

    for stream in socket.incoming() {
        let stream = match stream {
            Ok(t) => t,
            Err(t) => {
                eprintln!("[-]ERROR: {t}");
                continue;
            }
        };

        pool.execute(move || {
            handle_connection(stream);
        })
    }
}
