use std::time::{Duration, Instant};
use rand::Rng;

use rocksdb::{DB, Options};
use serde::Serialize;
use tokio::join;

/// Since sled is used for reading only during the initialization, large read cache is not required
const DB_CACHE_SIZE: usize = 10 * 1024 * 1024; // 10 mb

fn rocks_get<T: Serialize>(key: T, db: DB) {
    let key = bincode::serialize(&key).unwrap();
    match db.get(key) {
        Ok(Some(value)) => println!("retrieved value {}", String::from_utf8(value).unwrap()),
        Ok(None) => println!("value not found"),
        Err(e) => println!("operational problem encountered: {}", e),
    }
}

fn create_vec(n_dim: usize) -> Vec<f32> {
    let mut rng = rand::thread_rng();
    (0..n_dim).map(|_| rng.gen()).collect()
}


#[tokio::main]
async fn main() {

    // NB: db is automatically closed at end of lifetime
    let path = "_path_for_rocksdb_storage";
    
    let mut options: Options = Options::default();
    options.set_write_buffer_size(DB_CACHE_SIZE);
    options.create_if_missing(true);

    let options_t1 = options.clone();
    let options_t2 = options.clone();
    
    
    let t1 = tokio::spawn(async move{
        let db = DB::open(&options_t1, path).unwrap();
        for i in 1..10000 {
            let k = bincode::serialize(&i).unwrap();
            let v = k.clone();
            db.put(k, v).unwrap();
        }
    });


    let t2 = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let db = DB::open_for_read_only(&options_t2, path, false).unwrap();

        

        for i in 1..10000 {
            let val = db.get(bincode::serialize(&i).unwrap()).unwrap().unwrap();

            let vi: i32 = bincode::deserialize(&val).unwrap();

            //println!("K: {} V: {}", i, vi);
        }
        

    });
    
    let t0 = Instant::now();
    join!(t1, t2);

    println!("Elapsed: {:?}", t0.elapsed());



    let _ = DB::destroy(&Options::default(), path);
}

