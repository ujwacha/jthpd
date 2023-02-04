use serde::Deserialize;
use serde::Serialize;
use std::{
    env,
    io::Read,
    io::Write,
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread,
};

const DEFAULT_SEND_ADRESS: &'static str = "127.0.0.1:6969";
const DEFAULT_ADRESS: &'static str = "127.0.0.1:7979";
const DEFAULT_NO_OF_THREADS: usize = 8;

pub struct ThreadPool {
    threads: Vec<Worker>,
    sender: mpsc::Sender<Job>,
}

struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

pub fn get_addr_thread() -> (String, usize) {
    let adress = match env::var("JTHPD_ADRESS") {
        Ok(t) => t,
        Err(_) => String::from(DEFAULT_ADRESS), // if no value is provided, then the default will be this
    };

    let threads: usize = match env::var("JTHPD_MAX_PROCESS") {
        Ok(t) => t.parse().unwrap_or(DEFAULT_NO_OF_THREADS),
        Err(_) => DEFAULT_NO_OF_THREADS, // if the value is not provided default will be 8
    };

    (adress, threads)
}

pub fn print_help() {
    println!("This is the Help Page");
    println!("Environment variables:");
    println!(
        "JTHPD_ADRESS: for your listening adress default: {}",
        DEFAULT_SEND_ADRESS
    );
    println!(
        "JTHPD_MAX_PROCESS: number of threads: default {}",
        DEFAULT_NO_OF_THREADS
    );
    println!(
        "SEND_ADRESS: adress where hktcptsd is listeining to, default: {}",
        DEFAULT_ADRESS
    )
}

type Job = Box<dyn FnOnce() -> () + Send + 'static>;

impl ThreadPool {
    pub fn new(number: usize) -> Self {
        assert!(number > 0);

        let (sender, reciever) = mpsc::channel();

        let reciever = Arc::new(Mutex::new(reciever));

        let mut threads = Vec::with_capacity(number);

        for id in 0..number {
            let reciever = Arc::clone(&reciever);

            threads.push(Worker::new(id, reciever));
        }

        ThreadPool { threads, sender }
    }

    pub fn execute<F>(&self, job: F)
    where
        F: FnOnce() -> () + Send + 'static,
    {
        self.sender.send(Box::new(job)).unwrap(); // send the job to one of the workers
    }
}

impl Worker {
    fn new(id: usize, reciever: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let job = reciever // send through mutex, as it is moved. revieve a job from the mpsc challel which is a closure
                .lock()
                .unwrap()
                .recv()
                .unwrap();
            println!("thread {id} is working");
            job(); // and do the job
        });

        Worker { id, thread }
    }
}

pub fn handle_connection(stream: TcpStream) {
    let mut write_stream = stream.try_clone().unwrap();

    let mut buffer = [0; 1024 * 8];

    let last = match write_stream.read(&mut buffer) {
        Ok(t) => {
            println!("[+]Read to buffer, OK: {t}");
            t
        }

        Err(t) => {
            eprintln!("[-]Couldn't read to buffer, ERROR: {t}");
            panic!()
        }
    };

    let mut headers = [httparse::EMPTY_HEADER; 18];

    let mut req = httparse::Request::new(&mut headers);

    let res = match req.parse(&buffer).unwrap() {
        httparse::Status::Complete(t) => t,
        httparse::Status::Partial => {
            eprintln!("[-]Couldn't complete parsing");
            panic!()
        }
    };

    let bodyslice: String = buffer[res..last]
        .into_iter()
        .map(|a| *a as char)
        .collect::<String>();

    println!("{bodyslice}");
    send_to_hktcptsd(return_in_hktcptsd_protocol(bodyslice));
}

fn get_link_to_hktcptsd() -> String {
    match env::var("SEND_ADRESS") {
        Ok(t) => {
            println!("[+]Using adress from environment variable SEND_ADRESS: {t}");
            t
        }
        Err(t) => {
            eprintln!("[-]Didn't find environment variable SEND+ADRESS. ERROR:{t}");
            println!("[+]Using derault adress: {}", DEFAULT_SEND_ADRESS);
            DEFAULT_SEND_ADRESS.to_string()
        }
    }
}

fn send_to_hktcptsd(hktcptsd_req: String) {
    let mut stream =
        TcpStream::connect(get_link_to_hktcptsd()).expect("could not connect to hktcptsd");

    stream.write_all(hktcptsd_req.as_bytes()).unwrap();

    println!("sent to the hktcptsd");
}

fn return_in_hktcptsd_protocol(json: String) -> String {
    println!("{json}");
    let req: JsonRequest = serde_json::from_str(&json).unwrap();
    let retvalue = format!("{}\n{}\n{}\n", req.pass, req.id, req.string);
    println!("{retvalue}");
    retvalue
}
#[derive(Serialize, Deserialize, Debug)]
struct JsonRequest {
    pass: String,
    id: usize,
    string: String,
}
