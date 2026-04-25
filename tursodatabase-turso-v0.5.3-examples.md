# Code Examples for tursodatabase-turso (Version: v0.5.3)

## `doc_comment:bindings/java/rs_src/utils.rs:17:0`

```rust
set_err_msg_and_throw_exception(env, obj, Codes::SQLITE_ERROR, "An error occurred".to_string());
```
---
## `doc_comment:bindings/rust/src/params.rs:103:0`

```rust
# use turso::{Connection, params_from_iter, Rows};
# async fn run(conn: &Connection) {

let iter = vec![1, 2, 3];

conn.query(
"SELECT * FROM users WHERE id IN (?1, ?2, ?3)",
params_from_iter(iter)
)
.await
.unwrap();
# }
```
---
## `example_file:examples/rust/concurrent_writes.rs`

```rust
//! Concurrent writes with MVCC
//!
//! `BEGIN CONCURRENT` lets multiple connections write at the same time without
//! holding an exclusive lock.  Conflicts are detected at commit time: if two
//! transactions worked on same rows, the later one receives a conflict
//! error and must roll back and retry.

use rand::Rng;
use tempfile::NamedTempFile;
use turso::{Builder, Error};

fn is_retryable(e: &Error) -> bool {
    matches!(e, Error::Busy(_) | Error::BusySnapshot(_))
        || matches!(e, Error::Error(msg) if msg.contains("conflict"))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let tmp = NamedTempFile::new().expect("failed to create temp file");
    let db = Builder::new_local(tmp.path().to_str().unwrap())
        .build()
        .await?;

    let conn = db.connect()?;
    conn.pragma_update("journal_mode", "'mvcc'").await?;
    conn.execute("CREATE TABLE hits (val INTEGER)", ()).await?;

    let mut handles = Vec::new();
    for _ in 0..16 {
        let db = db.clone();
        handles.push(tokio::spawn(async move {
            let val = rand::rng().random_range(1..=100);
            let conn = db.connect()?;
            loop {
                conn.execute("BEGIN CONCURRENT", ()).await?;
                let result = conn
                    .execute(&format!("INSERT INTO hits VALUES ({val})"), ())
                    .await
                    .and(conn.execute("COMMIT", ()).await);
                match result {
                    Ok(_) => return Ok::<_, Error>(val),
                    Err(ref e) if is_retryable(e) => {
                        let _ = conn.execute("ROLLBACK", ()).await;
                        tokio::task::yield_now().await;
                    }
                    Err(e) => {
                        let _ = conn.execute("ROLLBACK", ()).await;
                        return Err(e);
                    }
                }
            }
        }));
    }

    for handle in handles {
        let val = handle.await.expect("task panicked")?;
        println!("inserted val={val}");
    }

    let mut rows = conn.query("SELECT COUNT(*) FROM hits", ()).await?;
    if let Some(row) = rows.next().await? {
        println!("total rows: {}", row.get::<i64>(0)?);
    }

    Ok(())
}

```
---
## `example_file:examples/rust/example.rs`

```rust
use turso::{Builder, Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("Turso Failed to Build memory db");

    let conn = db.connect()?;

    conn.query("select 1; select 1;", ()).await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (email TEXT, age INTEGER)",
        (),
    )
    .await?;

    conn.pragma_query("journal_mode", |row| {
        println!("{:?}", row.get_value(0));
        Ok(())
    })
    .await?;

    let mut stmt = conn
        .prepare("INSERT INTO users (email, age) VALUES (?1, ?2)")
        .await?;

    stmt.execute(["foo@example.com", &21.to_string()]).await?;

    let mut stmt = conn.prepare("SELECT * FROM users WHERE email = ?1").await?;

    let mut rows = stmt.query(["foo@example.com"]).await?;

    let row = rows.next().await?;

    assert!(
        row.is_some(),
        "The row that was just inserted hasn't been found"
    );

    if let Some(row_values) = row {
        let email = row_values.get_value(0)?;
        let age = row_values.get_value(1)?;
        println!("Row: {email:?} {age:?}");
    }

    Ok(())
}

```
---
## `example_file:examples/rust/example_struct.rs`

```rust
use turso::{transaction::Transaction, Builder, Connection, Error};

#[derive(Debug)]
struct User {
    email: String,
    age: i32,
}

async fn create_tables(conn: &Connection) -> Result<(), Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (email TEXT, age INTEGER)",
        (),
    )
    .await?;
    Ok(())
}

async fn insert_users(tx: &Transaction<'_>) -> Result<(), Error> {
    let mut stmt = tx
        .prepare("INSERT INTO users (email, age) VALUES (?1, ?2)")
        .await?;
    stmt.execute(["foo@example.com", &21.to_string()]).await?;
    stmt.execute(["bar@example.com", &22.to_string()]).await?;
    Ok(())
}

async fn list_users(conn: &Connection) -> Result<(), Error> {
    let mut stmt = conn
        .prepare("SELECT * FROM users WHERE email like ?1")
        .await?;

    let mut rows = stmt.query(["%@example.com"]).await?;

    while let Some(row) = rows.next().await? {
        let u: User = User {
            email: row.get(0)?,
            age: row.get(1)?,
        };
        println!("Row: {} {}", u.email, u.age);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let db = Builder::new_local(":memory:")
        .build()
        .await
        .expect("Turso Failed to Build memory db");

    let mut conn = db.connect()?;

    create_tables(&conn).await?;
    let tx = conn.transaction().await?;
    insert_users(&tx).await?;
    tx.commit().await?;
    list_users(&conn).await?;

    Ok(())
}

```
---
## `example_file:examples/rust/sync_example.rs`

```rust
//! Turso Database Sync example with Turso Cloud (with optional remote encryption)
//!
//! Environment variables:
//!   TURSO_REMOTE_URL              - Remote database URL (default: http://localhost:8080)
//!   TURSO_AUTH_TOKEN              - Auth token (optional)
//!   TURSO_REMOTE_ENCRYPTION_KEY   - Base64-encoded encryption key (optional)
//!   TURSO_REMOTE_ENCRYPTION_CIPHER - Cipher name (default: aes256gcm)
//!

use std::env;

use turso::sync::{Builder, RemoteEncryptionCipher};
use turso::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let remote_url =
        env::var("TURSO_REMOTE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let auth_token = env::var("TURSO_AUTH_TOKEN").ok();
    let encryption_key = env::var("TURSO_REMOTE_ENCRYPTION_KEY").ok();
    let encryption_cipher = env::var("TURSO_REMOTE_ENCRYPTION_CIPHER")
        .unwrap_or_else(|_| "aes256gcm".to_string())
        .parse::<RemoteEncryptionCipher>()
        .expect("invalid cipher");

    println!("Remote URL: {remote_url}");
    println!("Auth Token: {}", auth_token.is_some());
    println!("Encryption: {}", encryption_key.is_some());
    if encryption_key.is_some() {
        println!("Cipher: {encryption_cipher:?}");
    }

    let mut builder = Builder::new_remote(":memory:").with_remote_url(&remote_url);

    if let Some(token) = auth_token {
        builder = builder.with_auth_token(token);
    }

    if let Some(key) = encryption_key {
        builder = builder.with_remote_encryption(key, encryption_cipher);
    }

    let db = builder.build().await?;
    let conn = db.connect().await?;

    conn.execute("CREATE TABLE IF NOT EXISTS t (x TEXT)", ())
        .await?;

    let mut stmt = conn.prepare("SELECT COUNT(*) FROM t").await?;
    let mut rows = stmt.query(()).await?;
    let count: i64 = if let Some(row) = rows.next().await? {
        row.get(0)?
    } else {
        0
    };
    let next = count + 1;
    conn.execute(&format!("INSERT INTO t VALUES ('hello sync #{next}')"), ())
        .await?;
    db.push().await?;

    println!("\nTest table contents:");
    let mut stmt = conn.prepare("SELECT * FROM t").await?;
    let mut rows = stmt.query(()).await?;
    while let Some(row) = rows.next().await? {
        println!("  Row: {:?}", row.get_value(0)?);
    }

    // query sqlite_master for all tables
    println!("\nDatabase tables:");
    let mut stmt = conn
        .prepare("SELECT name, type FROM sqlite_master WHERE type='table'")
        .await?;
    let mut rows = stmt.query(()).await?;
    while let Some(row) = rows.next().await? {
        let name = row.get_value(0)?;
        let typ = row.get_value(1)?;
        println!("  - {typ:?}: {name:?}");
    }

    // sho database stats
    let stats = db.stats().await?;
    println!("\nDatabase stats:");
    println!("  Network received: {} bytes", stats.network_received_bytes);
    println!("  Network sent: {} bytes", stats.network_sent_bytes);
    println!("  Main WAL size: {} bytes", stats.main_wal_size);

    println!("\nDone!");
    Ok(())
}

```
---
## `readme:README.md:126:0`

```markdown
let db = Builder::new_local("sqlite.db").build().await?;
let conn = db.connect()?;

let res = conn.query("SELECT * FROM users", ()).await?;
```
---
## `readme:bindings/rust/README.md:121:2`

```markdown
use turso::sync::Builder;

#[tokio::main]
async fn main() -> turso::Result<()> {
    // Create a synced database
    let db = Builder::new_remote("local.db")
        .with_remote_url("libsql://your-database.turso.io")
        .with_auth_token("your-token")
        .build()
        .await?;

    let conn = db.connect().await?;

    // Create a table and insert data
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (id INTEGER PRIMARY KEY, content TEXT)",
        ()
    ).await?;

    conn.execute(
        "INSERT INTO notes (content) VALUES (?1)",
        ["My first synced note"]
    ).await?;

    // Push local changes to remote
    db.push().await?;

    // Pull remote changes to local
    db.pull().await?;

    Ok(())
}
```
---
## `readme:bindings/rust/README.md:162:3`

```markdown
let db = Builder::new_local(":memory:").build().await?;
let db = Builder::new_local("data.db").build().await?;
```
---
## `readme:bindings/rust/README.md:171:4`

```markdown
// Execute SQL directly
let rows_affected = conn.execute("INSERT INTO users (name) VALUES (?1)", ["Alice"]).await?;

// Query for multiple rows
let mut rows = conn.query("SELECT * FROM users WHERE age > ?1", [18]).await?;

// Prepare statements for reuse
let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?1").await?;
let mut rows = stmt.query([42]).await?;

// Execute prepared statements
let rows_affected = stmt.execute(["Alice"]).await?;
```
---
## `readme:bindings/rust/README.md:188:5`

```markdown
use futures_util::TryStreamExt;

let mut rows = conn.query("SELECT name, email FROM users", ()).await?;

while let Some(row) = rows.try_next().await? {
    let name = row.get_value(0)?.as_text().unwrap_or(&"".to_string());
    let email = row.get_value(1)?.as_text().unwrap_or(&"".to_string());
    println!("{}: {}", name, email);
}
```
---
## `readme:bindings/rust/README.md:206:6`

```markdown
use turso::sync::Builder;

let db = Builder::new_remote("local.db")       // Local database path (or ":memory:")
    .with_remote_url("libsql://db.turso.io")   // Remote URL (https://, http://, or libsql://)
    .with_auth_token("your-token")              // Authorization token
    .bootstrap_if_empty(true)                   // Download schema on first sync (default: true)
    .with_remote_encryption("base64-encoded-key", RemoteEncryptionCipher::Aes256Gcm) // Optional remote encryption
    .build()
    .await?;
```
---
## `readme:bindings/rust/README.md:222:7`

```markdown
// Push local changes to remote
db.push().await?;

// Pull remote changes (returns true if changes were applied)
let had_changes = db.pull().await?;

// Force WAL checkpoint
db.checkpoint().await?;

// Get sync statistics
let stats = db.stats().await?;
println!("Received: {} bytes", stats.network_received_bytes);
println!("Sent: {} bytes", stats.network_sent_bytes);
println!("WAL size: {} bytes", stats.main_wal_size);
```
---
## `readme:bindings/rust/README.md:39:0`

```markdown
use turso::Builder;

#[tokio::main]
async fn main() -> turso::Result<()> {
    // Create an in-memory database
    let db = Builder::new_local(":memory:").build().await?;
    let conn = db.connect()?;

    // Create a table
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
        ()
    ).await?;

    // Insert data
    conn.execute(
        "INSERT INTO users (name, email) VALUES (?1, ?2)",
        ["Alice", "alice@example.com"]
    ).await?;

    conn.execute(
        "INSERT INTO users (name, email) VALUES (?1, ?2)", 
        ["Bob", "bob@example.com"]
    ).await?;

    // Query data
    let mut rows = conn.query("SELECT * FROM users", ()).await?;
    
    while let Some(row) = rows.next().await? {
        let id = row.get_value(0)?;
        let name = row.get_value(1)?;
        let email = row.get_value(2)?;
        println!("User: {} - {} ({})", 
            id.as_integer().unwrap_or(&0), 
            name.as_text().unwrap_or(&"".to_string()), 
            email.as_text().unwrap_or(&"".to_string())
        );
    }

    Ok(())
}
```
---
## `readme:bindings/rust/README.md:85:1`

```markdown
use turso::Builder;

#[tokio::main] 
async fn main() -> turso::Result<()> {
    // Create or open a database file
    let db = Builder::new_local("my-database.db").build().await?;
    let conn = db.connect()?;

    // Create a table
    conn.execute(
        r#"CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )"#,
        ()
    ).await?;

    // Insert a post
    let rows_affected = conn.execute(
        "INSERT INTO posts (title, content) VALUES (?1, ?2)",
        ["Hello World", "This is my first blog post!"]
    ).await?;

    println!("Inserted {} rows", rows_affected);

    Ok(())
}
```
---
## `readme:extensions/core/README.md:122:2`

```markdown
use turso_ext::{register_extension, AggregateDerive, AggFunc, Value};
/// annotate your struct with the AggregateDerive macro, and it must implement the below AggFunc trait
#[derive(AggregateDerive)]
struct Percentile;

impl AggFunc for Percentile {
    /// The state to track during the steps
    type State = (Vec<f64>, Option<f64>, Option<String>); // Tracks the values, Percentile, and errors

    /// Define your error type, must impl Display
    type Error = String;

    /// Define the name you wish to call your function by. 
    /// e.g. SELECT percentile(value, 40);
     const NAME: &str = "percentile";

    /// Define the number of expected arguments for your function.
     const ARGS: i32 = 2;

    /// Define a function called on each row/value in a relevant group/column
    fn step(state: &mut Self::State, args: &[Value]) {
        let (values, p_value, error) = state;

        if let (Some(y), Some(p)) = (
            args.first().and_then(Value::to_float),
            args.get(1).and_then(Value::to_float),
        ) {
            if !(0.0..=100.0).contains(&p) {
                *error = Some("Percentile P must be between 0 and 100.".to_string());
                return;
            }

            if let Some(existing_p) = *p_value {
                if (existing_p - p).abs() >= 0.001 {
                    *error = Some("P values must remain consistent.".to_string());
                    return;
                }
            } else {
                *p_value = Some(p);
            }

            values.push(y);
        }
    }
    /// A function to finalize the state into a value to be returned as a result
    /// or an error (if you chose to track an error state as well)
    fn finalize(state: Self::State) -> Result<Value, Self::Error> {
        let (mut values, p_value, error) = state;

        if let Some(error) = error {
            return Err(error);
        }

        if values.is_empty() {
            return Ok(Value::null());
        }

        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = values.len() as f64;
        let p = p_value.unwrap();
        let index = (p * (n - 1.0) / 100.0).floor() as usize;

        Ok(Value::from_float(values[index]))
    }
}
```
---
## `readme:extensions/core/README.md:193:3`

```markdown
/// Example: A virtual table that operates on a CSV file as a database table.
/// This example assumes that the CSV file is located at "data.csv" in the current directory.
#[derive(Debug, VTabModuleDerive)]
struct CsvVTableModule;

impl VTabModule for CsvVTableModule {
    type Table = CsvTable;
    /// Declare the name for your virtual table
    const NAME: &'static str = "csv_data";
    /// Declare the type of vtable (TableValuedFunction or VirtualTable)
    const VTAB_KIND: VTabKind = VTabKind::VirtualTable;

    /// Declare your virtual table and its schema
    fn create(args: &[Value]) -> Result<(String, Self::Table), ResultCode> {
        let schema = "CREATE TABLE csv_data(
            name TEXT,
            age TEXT,
            city TEXT
        )".into();
        Ok((schema, CsvTable {}))
    }
}

struct CsvTable {}

impl VTable for CsvTable {
    type Cursor = CsvCursor;
    /// Define your error type. Must impl Display and match Cursor::Error
    type Error = &'static str;

    /// Open to return a new cursor: In this simple example, the CSV file is read completely into memory on connect.
    fn open(&self, conn: Option<Rc<Connection>>) -> Result<Self::Cursor, Self::Error> {
        // Read CSV file contents from "data.csv"
        let csv_content = fs::read_to_string("data.csv").unwrap_or_default();
        // For simplicity, we'll ignore the header row.
        let rows: Vec<Vec<String>> = csv_content
            .lines()
            .skip(1)
            .map(|line| {
                line.split(',')
                    .map(|s| s.trim().to_string())
                    .collect()
            })
            .collect();
        // store the connection for later use. Connection is Option to allow writing tests for your module
        // but will be available to use by storing on your Cursor implementation
        Ok(CsvCursor { rows, index: 0, connection: conn.unwrap() })
    }

    /// *Optional* methods for non-readonly tables

    /// Update the value at rowid
    fn update(&mut self, _rowid: i64, _args: &[Value]) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Insert the value(s)
    fn insert(&mut self, _args: &[Value]) -> Result<i64, Self::Error> {
        Ok(0)
    }
    /// Delete the value at rowid
    fn delete(&mut self, _rowid: i64) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The cursor for iterating over CSV rows.
#[derive(Debug)]
struct CsvCursor {
    rows: Vec<Vec<String>>,
    index: usize,
    connection: Rc<Connection>,
}

/// Implement the VTabCursor trait for your cursor type
impl VTabCursor for CsvCursor {
    type Error = &'static str;

    /// Filter through result columns. (not used in this simple example)
    fn filter(&mut self, args: &[Value], _idx_info: Option<(&str, i32)>) -> ResultCode {
        ResultCode::OK
    }

    /// Next advances the cursor to the next row.
    fn next(&mut self) -> ResultCode {
        if self.index < self.rows.len() - 1 {
            self.index += 1;
            ResultCode::OK
        } else {
            ResultCode::EOF
        }
    }

    /// Return true if the cursor is at the end.
    fn eof(&self) -> bool {
        self.index >= self.rows.len()
    }

    /// Return the value for the column at the given index in the current row.
    fn column(&self, idx: u32) -> Result<Value, Self::Error> {
        let row = &self.rows[self.index];
        if (idx as usize) < row.len() {
            Ok(Value::from_text(&row[idx as usize]))
        } else {
            Ok(Value::null())
        }
    }

    fn rowid(&self) -> i64 {
        self.index as i64
    }
}
```
---
## `readme:extensions/core/README.md:314:4`

```markdown
let mut stmt = self.connection.prepare("SELECT col FROM table where name = ?;");
 stmt.bind_at(NonZeroUsize::new(1).unwrap(), args[0]);

 /// use the connection similarly to the API of the core library
 while let StepResult::Row = stmt.step() {
       let row = stmt.get_row();
       if let Some(val) = row.first() {
           // access values
           println!("result: {:?}", val);
       }
   }
  stmt.close();

  if let Ok(Some(last_insert_rowid)) = conn.execute("INSERT INTO table (col, name) VALUES ('test', 'data')") {
      println!("rowid of insert: {:?}", last_insert_rowid);
  }
```
---
## `readme:extensions/core/README.md:341:5`

```markdown
use turso_ext::{ExtResult, VfsDerive, VfsExtension, VfsFile, Callback};

/// Your struct must also impl Default
#[derive(VfsDerive, Default)]
pub struct TestFS {
    callbacks: CallbackQueue,
}

impl VfsExtension for TestFS {
    const NAME: &'static str = "testvfs";
    type File = TestFile;
    fn run_once(&self) -> ExtResult<()> {
        log::debug!("running once with testing VFS");
        self.callbacks.process_all();
        Ok(())
    }

    fn open_file(&self, path: &str, flags: i32, _direct: bool) -> ExtResult<Self::File> {
        let _ = env_logger::try_init();
        log::debug!("opening file with testing VFS: {path} flags: {flags}");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(flags & 1 != 0)
            .open(path)
            .map_err(|_| ResultCode::Error)?;
        Ok(TestFile {
            file,
            io: self.callbacks.clone(),
        })
    }

    fn remove_file(&self, path: &str) -> ExtResult<()> {
        let _ = env_logger::try_init();
        log::debug!("remove file with testing VFS: {path}");
        std::fs::remove_file(path).map_err(|_| ResultCode::Error)
    }
}

impl VfsFile for TestFile {
    fn read(&mut self, mut buf: BufferRef, offset: i64, cb: Callback) -> ExtResult<()> {
        log::debug!(
            "reading file with testing VFS: bytes: {} offset: {}",
            buf.len(),
            offset
        );
        if self.file.seek(SeekFrom::Start(offset as u64)).is_err() {
            return Err(ResultCode::Error);
        }
        let len = buf.len();
        let buf = buf.as_mut_slice();
        let res = self
            .file
            .read(&mut buf[..len])
            .map_err(|_| ResultCode::Error)
            .map(|n| n as i32)?;
        self.io.enqueue(cb, res);
        Ok(())
    }

    fn write(&mut self, buf: turso_ext::BufferRef, offset: i64, cb: Callback) -> ExtResult<()> {
        log::debug!(
            "writing to file with testing VFS: bytes: {} offset: {offset}",
            buf.len()
        );
        if self.file.seek(SeekFrom::Start(offset as u64)).is_err() {
            return Err(ResultCode::Error);
        }
        let len = buf.len();
        let n = self
            .file
            .write(&buf[..len])
            .map_err(|_| ResultCode::Error)
            .map(|n| n as i32)?;
        self.io.enqueue(cb, n);
        Ok(())
    }

    fn sync(&self, cb: Callback) -> ExtResult<()> {
        log::debug!("syncing file with testing VFS");
        self.file.sync_all().map_err(|_| ResultCode::Error)?;
        self.io.enqueue(cb, 0);
        Ok(())
    }

    fn truncate(&self, len: i64, cb: Callback) -> ExtResult<()> {
        log::debug!("truncating file with testing VFS to length: {len}");
        self.file
            .set_len(len as u64)
            .map_err(|_| ResultCode::Error)?;
        self.io.enqueue(cb, 0);
        Ok(())
    }

    fn size(&self) -> i64 {
        self.file.metadata().map(|m| m.len() as i64).unwrap_or(-1)
    }
}
```
---
## `readme:extensions/core/README.md:81:0`

```markdown
register_extension!{
    scalars: { double }, // name of your function, if different from attribute name
    aggregates: { Percentile },
    vtabs: { CsvVTable },
    vfs: { ExampleFS },
}
```
---
## `readme:extensions/core/README.md:95:1`

```markdown
use turso_ext::{register_extension, Value, scalar};

/// Annotate each with the scalar macro, specifying the name you would like to call it with
/// and optionally, an alias.. e.g. SELECT double(4); or SELECT twice(4);
#[scalar(name = "double", alias = "twice")]
fn double(&self, args: &[Value]) -> Value {
    if let Some(arg) = args.first() {
        match arg.value_type() {
            ValueType::Float => {
                let val = arg.to_float().unwrap();
                Value::from_float(val * 2.0)
            }
            ValueType::Integer => {
                let val = arg.to_integer().unwrap();
                Value::from_integer(val * 2)
            }
        }
    } else {
        Value::null()
    }
}
```
---
## `readme:sdk-kit/README.md:16:0`

```markdown
use turso_sdk_kit::rsapi::{
    TursoDatabase, TursoDatabaseConfig, TursoStatusCode, Value, ValueRef,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the database holder (not opened yet).
    let db = TursoDatabase::create(TursoDatabaseConfig {
        path: ":memory:".to_string(),
        experimental_features: None,
        io: None,
        // When true, step/execute may return Io and you should call run_io() to progress.
        async_io: true,
    });

    // Open and connect.
    db.open()?;
    let conn = db.connect()?;

    // Prepare, bind, and step a simple query.
    let mut stmt = conn.prepare_single("SELECT :greet || ' Turso'")?;
    stmt.bind_named("greet", Value::Text("Hello".into()))?;

    loop {
        match stmt.step()? {
            TursoStatusCode::Row => {
                // Read current row value. Valid until next step/reset/finalize.
                match stmt.row_value(0)? {
                    ValueRef::Text(t) => println!("{}", t.as_str()),
                    other => println!("row[0] = {:?}", other),
                }
            }
            TursoStatusCode::Io => {
                // Drive one iteration of the I/O backend.
                stmt.run_io()?;
            }
            TursoStatusCode::Done => break,
            _ => unreachable!("unexpected status"),
        }
    }

    // Finalize to complete the statement cleanly (may also return Io).
    match stmt.finalize()? {
        TursoStatusCode::Io => {
            // If needed, drive IO and finalize again.
            stmt.run_io()?;
            let _ = stmt.finalize()?;
        }
        _ => {}
    }

    Ok(())
}
```
---
## `readme:testing/runner/docs/README.md:114:0`

```markdown
trait SqlBackend {
    fn create_database(&self, config: &DatabaseConfig)
        -> Result<Box<dyn DatabaseInstance>>;
}

trait DatabaseInstance {
    async fn execute(&mut self, sql: &str) -> Result<QueryResult>;
    async fn close(self: Box<Self>) -> Result<()>;
}
```
---
## `readme:testing/simulator/README.md:74:0`

```markdown
/// Insert-Select is a property in which the inserted row
/// must be in the resulting rows of a select query that has a
/// where clause that matches the inserted row.
/// The execution of the property is as follows
///     INSERT INTO <t> VALUES (...)
///     I_0
///     I_1
///     ...
///     I_n
///     SELECT * FROM <t> WHERE <predicate>
/// The interactions in the middle has the following constraints;
/// - There will be no errors in the middle interactions.
/// - The inserted row will not be deleted.
/// - The inserted row will not be updated.
/// - The table `t` will not be renamed, dropped, or altered.
InsertValuesSelect {
    /// The insert query
    insert: Insert,
    /// Selected row index
    row_index: usize,
    /// Additional interactions in the middle of the property
    queries: Vec<Query>,
    /// The select query
    select: Select,
},
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_cacheflush`

```rust
let builder = Builder::new_local("test.db");
    let db = builder.build().await.unwrap();

    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE IF NOT EXISTS asdf (x INTEGER)", ())
        .await
        .unwrap();

    // Tests if cache flush breaks transaction isolation
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO asdf (x) VALUES (1)", ())
        .await
        .unwrap();
    conn.cacheflush().unwrap();
    conn.execute("ROLLBACK", ()).await.unwrap();

    conn.execute("INSERT INTO asdf (x) VALUES (2)", ())
        .await
        .unwrap();
    conn.execute("INSERT INTO asdf (x) VALUES (3)", ())
        .await
        .unwrap();

    let mut res = conn.query("SELECT * FROM asdf", ()).await.unwrap();

    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        2.into()
    );
    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        3.into()
    );

    // Tests if cache flush doesn't break a committed transaction
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO asdf (x) VALUES (1)", ())
        .await
        .unwrap();
    conn.cacheflush().unwrap();
    conn.execute("COMMIT", ()).await.unwrap();

    let mut res = conn
        .query("SELECT * FROM asdf WHERE x = 1", ())
        .await
        .unwrap();

    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        1.into()
    );

    fs::remove_file("test.db").await.unwrap();
    fs::remove_file("test.db-wal").await.unwrap();
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_check_on_conflict_abort`

```rust
// ABORT (default): error on the violating statement, transaction stays active.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE t(id INTEGER PRIMARY KEY, value INTEGER CHECK(value > 0))",
        (),
    )
    .await
    .unwrap();
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO t VALUES(1, 10)", ())
        .await
        .unwrap();

    let err = conn
        .execute("INSERT OR ABORT INTO t VALUES(2, -5)", ())
        .await;
    assert!(
        err.is_err(),
        "INSERT OR ABORT should error on CHECK violation"
    );

    conn.execute("COMMIT", ()).await.unwrap();

    let ids = collect_ids(&conn, "SELECT id FROM t ORDER BY id").await;
    assert_eq!(ids, vec![1]);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_check_on_conflict_fail`

```rust
// FAIL: error on the violating statement, transaction stays active.
    // Prior inserts within the transaction are preserved and can be committed.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE t(id INTEGER PRIMARY KEY, value INTEGER CHECK(value > 0))",
        (),
    )
    .await
    .unwrap();
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO t VALUES(1, 10)", ())
        .await
        .unwrap();

    // This should fail but keep the transaction active
    let err = conn
        .execute("INSERT OR FAIL INTO t VALUES(2, -5)", ())
        .await;
    assert!(
        err.is_err(),
        "INSERT OR FAIL should error on CHECK violation"
    );

    // Transaction is still active — commit it
    conn.execute("COMMIT", ()).await.unwrap();

    // Row 1 should have survived
    let ids = collect_ids(&conn, "SELECT id FROM t ORDER BY id").await;
    assert_eq!(ids, vec![1]);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_check_on_conflict_replace`

```rust
// REPLACE: for CHECK constraints, behaves like ABORT.
    // Error, transaction stays active.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE t(id INTEGER PRIMARY KEY, value INTEGER CHECK(value > 0))",
        (),
    )
    .await
    .unwrap();
    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO t VALUES(1, 10)", ())
        .await
        .unwrap();

    let err = conn
        .execute("INSERT OR REPLACE INTO t VALUES(1, -5)", ())
        .await;
    assert!(
        err.is_err(),
        "INSERT OR REPLACE should error on CHECK violation"
    );

    conn.execute("COMMIT", ()).await.unwrap();

    let ids = collect_ids(&conn, "SELECT id FROM t ORDER BY id").await;
    assert_eq!(ids, vec![1]);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_check_on_conflict_rollback`

```rust
// ROLLBACK: rolls back the entire transaction.
    // Prior inserts within the transaction are lost, but committed rows survive.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE t(id INTEGER PRIMARY KEY, value INTEGER CHECK(value > 0))",
        (),
    )
    .await
    .unwrap();
    // Commit row 1 outside the transaction
    conn.execute("INSERT INTO t VALUES(1, 10)", ())
        .await
        .unwrap();

    conn.execute("BEGIN", ()).await.unwrap();
    conn.execute("INSERT INTO t VALUES(2, 20)", ())
        .await
        .unwrap();

    // This should fail AND roll back the transaction
    let err = conn
        .execute("INSERT OR ROLLBACK INTO t VALUES(3, -5)", ())
        .await;
    assert!(
        err.is_err(),
        "INSERT OR ROLLBACK should error on CHECK violation"
    );

    // Transaction was rolled back — row 2 is lost, row 1 survives
    let ids = collect_ids(&conn, "SELECT id FROM t ORDER BY id").await;
    assert_eq!(ids, vec![1]);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_connection_clone`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let mut conn = db.connect().unwrap();

    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)", ())
        .await
        .unwrap();

    let tx = conn.transaction().await.unwrap();
    let mut stmt = tx
        .prepare("INSERT INTO users VALUES (?1, ?2)")
        .await
        .unwrap();
    stmt.execute(["1", "Frodo"]).await.unwrap();
    tx.commit().await.unwrap();

    let conn2 = conn.clone();
    let row = conn2
        .prepare("SELECT id FROM users WHERE name = ?")
        .await
        .unwrap()
        .query_row(&["Frodo"])
        .await
        .unwrap();

    let id: i64 = row.get(0).unwrap();
    assert_eq!(id, 1);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_encryption`

```rust
let temp_dir = tempfile::tempdir().unwrap();
    let db_file = temp_dir.path().join("test-encrypted.db");
    let db_file = db_file.to_str().unwrap();
    let hexkey = "b1bbfda4f589dc9daaf004fe21111e00dc00c98237102f5c7002a5669fc76327";
    let wrong_key = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let encryption_opts = EncryptionOpts {
        hexkey: hexkey.to_string(),
        cipher: "aegis256".to_string(),
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_index`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE users (name TEXT PRIMARY KEY, email TEXT)", ())
        .await
        .unwrap();
    conn.execute("CREATE INDEX email_idx ON users(email)", ())
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO users VALUES ('alice', 'a@b.c'), ('bob', 'b@d.e')",
        (),
    )
    .await
    .unwrap();

    let mut rows = conn
        .query("SELECT * FROM users WHERE email = 'a@b.c'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(row.get::<String>(0).unwrap() == "alice");
    assert!(row.get::<String>(1).unwrap() == "a@b.c");
    assert!(rows.next().await.unwrap().is_none());

    let mut rows = conn
        .query("SELECT * FROM users WHERE email = 'b@d.e'", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert!(row.get::<String>(0).unwrap() == "bob");
    assert!(row.get::<String>(1).unwrap() == "b@d.e");
    assert!(rows.next().await.unwrap().is_none());
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_insert_returning_partial_consume`

```rust
// Regression test for: INSERT...RETURNING should insert all rows even if
    // only some RETURNING values are consumed before the statement is dropped/reset.
    // This matches the sqlite3 bindings fix in commit e39e60ef1.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (x INTEGER)", ())
        .await
        .unwrap();

    // Use query() to get RETURNING values, but only consume first row
    let mut stmt = conn
        .prepare("INSERT INTO t (x) VALUES (1), (2), (3) RETURNING x")
        .await
        .unwrap();
    let mut rows = stmt.query(()).await.unwrap();

    // Only consume first row
    let first_row = rows.next().await.unwrap().unwrap();
    assert_eq!(first_row.get::<i64>(0).unwrap(), 1);

    // Drop the rows iterator without consuming remaining rows
    drop(rows);
    drop(stmt);

    // All 3 rows should have been inserted despite only consuming 1 RETURNING value
    let mut count_rows = conn.query("SELECT COUNT(*) FROM t", ()).await.unwrap();
    let count: i64 = count_rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(
        count, 3,
        "All 3 rows should be inserted even if RETURNING was partially consumed"
    );
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_once_not_cleared_on_reset_with_coroutine`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    // This query generates bytecode with Once inside a coroutine:
    // The outer FROM-clause subquery creates a coroutine, and the inner
    // scalar subquery (SELECT 1) uses Once to evaluate only once per execution.
    let mut stmt = conn
        .prepare("SELECT * FROM (SELECT (SELECT 1))")
        .await
        .unwrap();

    let mut rows = stmt.query(()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let value: i64 = row.get(0).unwrap();
    assert_eq!(value, 1);
    assert!(rows.next().await.unwrap().is_none());
    drop(rows);

    stmt.reset().unwrap();

    let mut rows = stmt.query(()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();

    assert_eq!(
        row.get_value(0).unwrap(),
        Value::Integer(1),
        "Second execution should return 1, not Null. Bug: state.once not cleared in reset()"
    );
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_basic`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", ())
        .await
        .unwrap();

    // First call should cache the statement
    let mut stmt1 = conn
        .prepare_cached("SELECT * FROM users WHERE id = ?")
        .await
        .unwrap();

    // Insert some data and query
    conn.execute("INSERT INTO users VALUES (1, 'Alice')", ())
        .await
        .unwrap();

    let mut rows = stmt1.query(vec![Value::Integer(1)]).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i64>(0).unwrap(), 1);
    assert_eq!(row.get::<String>(1).unwrap(), "Alice");
    drop(rows);
    drop(stmt1);

    // Second call should use cached statement
    let mut stmt2 = conn
        .prepare_cached("SELECT * FROM users WHERE id = ?")
        .await
        .unwrap();

    let mut rows = stmt2.query(vec![Value::Integer(1)]).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i64>(0).unwrap(), 1);
    assert_eq!(row.get::<String>(1).unwrap(), "Alice");
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_batch_insert_delete_pattern`

```rust
#[derive(Clone)]
    struct Host {
        name: String,
        app: String,
        address: String,
        namespace: String,
        cloud_cluster_name: String,
        allowed_ips: Vec<String>,
        updated_at: std::time::SystemTime,
        deleted: bool,
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_independent_state`

```rust
// Verify that each cached statement has independent execution state
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (id INTEGER)", ())
        .await
        .unwrap();

    for i in 1..=5 {
        conn.execute(&format!("INSERT INTO t VALUES ({i
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_multiple_statements`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (id INTEGER, value TEXT)", ())
        .await
        .unwrap();

    // Cache multiple different statements
    let queries = vec![
        "SELECT * FROM t WHERE id = ?",
        "SELECT * FROM t WHERE value = ?",
        "INSERT INTO t VALUES (?, ?)",
    ];

    for query in &queries {
        let _ = conn.prepare_cached(*query).await.unwrap();
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_reprepare_on_query_only_change`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (id INTEGER)", ())
        .await
        .unwrap();

    let mut stmt = conn
        .prepare_cached("INSERT INTO t VALUES (?)")
        .await
        .unwrap();

    conn.execute("PRAGMA query_only=1", ()).await.unwrap();

    let err = stmt.execute(vec![Value::Integer(1)]).await.unwrap_err();
    assert!(err.to_string().to_ascii_lowercase().contains("query_only"));

    let mut rows = conn.query("SELECT COUNT(*) FROM t", ()).await.unwrap();
    let count: i64 = rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 0);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_stress`

```rust
// Stress test to ensure cache works correctly under repeated use
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, value TEXT)", ())
        .await
        .unwrap();

    let insert_query = "INSERT INTO t (id, value) VALUES (?, ?)";
    let select_query = "SELECT value FROM t WHERE id = ?";

    // Insert many rows using cached statement
    for i in 0..100 {
        let mut stmt = conn.prepare_cached(insert_query).await.unwrap();
        stmt.execute(vec![Value::Integer(i), Value::Text(format!("value_{i
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_cached_with_parameters`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute(
        "CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)",
        (),
    )
    .await
    .unwrap();

    conn.execute("INSERT INTO users VALUES (1, 'Alice', 30)", ())
        .await
        .unwrap();
    conn.execute("INSERT INTO users VALUES (2, 'Bob', 25)", ())
        .await
        .unwrap();
    conn.execute("INSERT INTO users VALUES (3, 'Charlie', 35)", ())
        .await
        .unwrap();

    let query = "SELECT name FROM users WHERE age > ?";

    // Use cached statement with different parameters
    let mut stmt = conn.prepare_cached(query).await.unwrap();

    let mut rows = stmt.query(vec![Value::Integer(25)]).await.unwrap();
    let mut names = Vec::new();
    while let Some(row) = rows.next().await.unwrap() {
        names.push(row.get::<String>(0).unwrap());
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_prepare_vs_prepare_cached_equivalence`

```rust
// Verify that prepare_cached produces same results as prepare
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (x INTEGER, y TEXT)", ())
        .await
        .unwrap();

    conn.execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')", ())
        .await
        .unwrap();

    let query = "SELECT * FROM t ORDER BY x";

    // Results from prepare
    let mut stmt1 = conn.prepare(query).await.unwrap();
    let mut rows1 = stmt1.query(()).await.unwrap();
    let mut results1 = Vec::new();
    while let Some(row) = rows1.next().await.unwrap() {
        results1.push((row.get::<i64>(0).unwrap(), row.get::<String>(1).unwrap()));
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_query_row_returns_first_row`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)", ())
        .await
        .unwrap();

    conn.execute("INSERT INTO users VALUES (1, 'Frodo')", ())
        .await
        .unwrap();

    let row = conn
        .prepare("SELECT id FROM users WHERE name = ?")
        .await
        .unwrap()
        .query_row(&["Frodo"])
        .await
        .unwrap();

    let id: i64 = row.get(0).unwrap();
    assert_eq!(id, 1);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_query_row_returns_no_rows_error`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)", ())
        .await
        .unwrap();

    let result = conn
        .prepare("SELECT id FROM users WHERE name = ?")
        .await
        .unwrap()
        .query_row(&["Ghost"])
        .await;

    assert!(matches!(result, Err(Error::QueryReturnedNoRows)));
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_row_get_column_typed`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE v (n INTEGER, label TEXT)", ())
        .await
        .unwrap();

    conn.execute("INSERT INTO v VALUES (42, 'answer')", ())
        .await
        .unwrap();

    let mut rows = conn.query("SELECT * FROM v", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();

    let n: i64 = row.get(0).unwrap();
    let label: String = row.get(1).unwrap();

    assert_eq!(n, 42);
    assert_eq!(label, "answer");
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_row_get_conversion_error`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (x TEXT)", ()).await.unwrap();

    conn.execute("INSERT INTO t VALUES (NULL)", ())
        .await
        .unwrap();

    let mut rows = conn.query("SELECT x FROM t", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();

    // Attempt to convert TEXT into integer (should fail)
    let result: Result<u32, _> = row.get(0);
    assert!(matches!(result, Err(Error::ConversionFailure(_))));
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_row_get_value_out_of_bounds`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (x INTEGER)", ())
        .await
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1)", ()).await.unwrap();

    let mut rows = conn.query("SELECT x FROM t", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();

    // Valid index works
    assert!(row.get_value(0).is_ok());

    // Out of bounds returns error instead of panicking
    let result = row.get_value(999);
    assert!(matches!(result, Err(Error::Misuse(_))));

    // Also test get<T>() for OOB
    let result: Result<i64, _> = row.get(999);
    assert!(matches!(result, Err(Error::Misuse(_))));
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_rows_next`

```rust
let builder = Builder::new_local(":memory:");
    let db = builder.build().await.unwrap();
    let conn = db.connect().unwrap();
    conn.execute("CREATE TABLE test (x INTEGER)", ())
        .await
        .unwrap();
    conn.execute("INSERT INTO test (x) VALUES (1)", ())
        .await
        .unwrap();
    assert_eq!(conn.last_insert_rowid(), 1);
    conn.execute("INSERT INTO test (x) VALUES (2)", ())
        .await
        .unwrap();
    assert_eq!(conn.last_insert_rowid(), 2);
    conn.execute(
        "INSERT INTO test (x) VALUES (:x)",
        vec![(":x".to_string(), Value::Integer(3))],
    )
    .await
    .unwrap();
    assert_eq!(conn.last_insert_rowid(), 3);
    conn.execute(
        "INSERT INTO test (x) VALUES (@x)",
        vec![("@x".to_string(), Value::Integer(4))],
    )
    .await
    .unwrap();
    assert_eq!(conn.last_insert_rowid(), 4);
    conn.execute(
        "INSERT INTO test (x) VALUES ($x)",
        vec![("$x".to_string(), Value::Integer(5))],
    )
    .await
    .unwrap();
    assert_eq!(conn.last_insert_rowid(), 5);
    let mut res = conn.query("SELECT * FROM test", ()).await.unwrap();
    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        1.into()
    );
    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        2.into()
    );
    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        3.into()
    );
    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        4.into()
    );
    assert_eq!(
        res.next().await.unwrap().unwrap().get_value(0).unwrap(),
        5.into()
    );
    assert!(res.next().await.unwrap().is_none());
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_rows_returned`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    //--- CRUD Operations ---//
    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)", ())
        .await
        .unwrap();
    let changed = conn
        .execute("INSERT INTO t VALUES (1,'hello')", ())
        .await
        .unwrap();
    let changed1 = conn
        .execute("INSERT INTO t VALUES (2,'hi')", ())
        .await
        .unwrap();
    let changed2 = conn
        .execute("UPDATE t SET val='hi' WHERE id=1", ())
        .await
        .unwrap();
    let changed3 = conn
        .execute("DELETE FROM t WHERE val='hi'", ())
        .await
        .unwrap();
    assert_eq!(changed, 1);
    assert_eq!(changed1, 1);
    assert_eq!(changed2, 1);
    assert_eq!(changed3, 2);

    //--- A more complicated example of insert with a select join subquery ---//
    conn.execute(
        "CREATE TABLE authors ( id INTEGER PRIMARY KEY, name TEXT NOT NULL);
       ",
        (),
    )
    .await
    .unwrap();

    conn.execute(
       "CREATE TABLE books ( id INTEGER PRIMARY KEY, author_id INTEGER NOT NULL REFERENCES authors(id), title TEXT NOT NULL); "
       ,()
   ).await.unwrap();

    conn.execute(
        "CREATE TABLE prize_winners ( book_id INTEGER PRIMARY KEY, author_name TEXT NOT NULL);",
        (),
    )
    .await
    .unwrap();

    conn.execute(
        "INSERT INTO authors (id, name) VALUES (1, 'Alice'), (2, 'Bob');",
        (),
    )
    .await
    .unwrap();

    conn.execute(
       "INSERT INTO books (id, author_id, title) VALUES (1, 1, 'Rust in Action'), (2, 1, 'Async Adventures'), (3, 1, 'Fearless Concurrency'), (4, 1, 'Unsafe Tales'), (5, 1, 'Zero-Cost Futures'), (6, 2, 'Learning SQL');",
       ()
   ).await.unwrap();

    let rows_changed = conn
        .execute(
            "
       INSERT INTO prize_winners (book_id, author_name)
       SELECT b.id, a.name
       FROM   books b
       JOIN   authors a ON a.id = b.author_id
       WHERE  a.id = 1;       -- Alice's five books
       ",
            (),
        )
        .await
        .unwrap();

    assert_eq!(rows_changed, 5);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_statement_query_resets_before_execution`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, value TEXT)", ())
        .await
        .unwrap();

    for i in 0..5 {
        conn.execute(&format!("INSERT INTO t VALUES ({i
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_strict_tables`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    // Create a STRICT table
    conn.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT) STRICT",
        (),
    )
    .await
    .unwrap();

    // Insert valid data
    conn.execute("INSERT INTO users VALUES (1, 'Alice')", ())
        .await
        .unwrap();

    // Query the data
    let mut rows = conn.query("SELECT id, name FROM users", ()).await.unwrap();
    let row = rows.next().await.unwrap().unwrap();
    assert_eq!(row.get::<i64>(0).unwrap(), 1);
    assert_eq!(row.get::<String>(1).unwrap(), "Alice");
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_transaction_commit_without_mvcc`

```rust
// Regression test: COMMIT should work for non-MVCC transactions.
    // The op_auto_commit function must check TransactionState, not just MVCC tx.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", ())
        .await
        .unwrap();

    // Begin explicit transaction
    conn.execute("BEGIN IMMEDIATE TRANSACTION", ())
        .await
        .unwrap();

    // Insert data within transaction
    conn.execute("INSERT INTO test (id, value) VALUES (1, 'hello')", ())
        .await
        .unwrap();

    // Commit should succeed
    conn.execute("COMMIT", ())
        .await
        .expect("COMMIT should succeed for non-MVCC transactions");

    // Verify data was committed
    let mut rows = conn
        .query("SELECT value FROM test WHERE id = 1", ())
        .await
        .unwrap();
    let row = rows.next().await.unwrap().unwrap();
    let value: String = row.get(0).unwrap();
    assert_eq!(value, "hello", "Data should be committed");
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_transaction_prepared_statement`

```rust
let db = Builder::new_local(":memory:").build().await.unwrap();
    let mut conn = db.connect().unwrap();

    conn.execute("CREATE TABLE users (id INTEGER, name TEXT)", ())
        .await
        .unwrap();

    let tx = conn.transaction().await.unwrap();
    let mut stmt = tx
        .prepare("INSERT INTO users VALUES (?1, ?2)")
        .await
        .unwrap();
    stmt.execute(["1", "Frodo"]).await.unwrap();
    tx.commit().await.unwrap();

    let row = conn
        .prepare("SELECT id FROM users WHERE name = ?")
        .await
        .unwrap()
        .query_row(&["Frodo"])
        .await
        .unwrap();

    let id: i64 = row.get(0).unwrap();
    assert_eq!(id, 1);
```
---
## `test:bindings/rust/tests/integration_tests.rs:test_transaction_with_insert_returning_then_commit`

```rust
// Regression test: Combining INSERT...RETURNING (partial consume) with explicit transaction.
    // This tests the interaction between the reset-to-completion fix and transaction commit.
    let db = Builder::new_local(":memory:").build().await.unwrap();
    let conn = db.connect().unwrap();

    conn.execute("CREATE TABLE t (x INTEGER)", ())
        .await
        .unwrap();

    // Begin transaction
    conn.execute("BEGIN IMMEDIATE TRANSACTION", ())
        .await
        .unwrap();

    // INSERT...RETURNING, only consume first row
    let mut stmt = conn
        .prepare("INSERT INTO t (x) VALUES (1), (2), (3) RETURNING x")
        .await
        .unwrap();
    let mut rows = stmt.query(()).await.unwrap();
    let first = rows.next().await.unwrap().unwrap();
    assert_eq!(first.get::<i64>(0).unwrap(), 1);
    drop(rows);
    drop(stmt);

    // Commit should succeed even after partial RETURNING consumption
    conn.execute("COMMIT", ())
        .await
        .expect("COMMIT should succeed after INSERT...RETURNING");

    // Verify all 3 rows were inserted
    let mut count_rows = conn.query("SELECT COUNT(*) FROM t", ()).await.unwrap();
    let count: i64 = count_rows.next().await.unwrap().unwrap().get(0).unwrap();
    assert_eq!(count, 3, "All rows should be committed");
```
---
## `test:cli/tests/non_interactive_exit_code.rs:piped_stdin_empty_returns_zero`

```rust
let mut child = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run tursodb");

    // Close stdin immediately — no input
    drop(child.stdin.take());

    let status = child.wait().expect("failed to wait");
    assert_eq!(status.code(), Some(0));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:piped_stdin_returns_exit_code_one_on_query_failure`

```rust
let mut child = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run tursodb");

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"select * from nonexistent;\n").unwrap();
    drop(stdin);

    let status = child.wait().expect("failed to wait");
    assert_eq!(status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:piped_stdin_returns_exit_code_zero_on_success`

```rust
let mut child = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run tursodb");

    let mut stdin = child.stdin.take().unwrap();
    stdin.write_all(b"select 1;\n").unwrap();
    drop(stdin);

    let status = child.wait().expect("failed to wait");
    assert_eq!(status.code(), Some(0));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:piped_stdin_runtime_error_returns_nonzero`

```rust
let mut child = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run tursodb");

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(
            b"create table t(x integer primary key);\n\
              insert into t values(1);\n\
              insert into t values(1);\n",
        )
        .unwrap();
    drop(stdin);

    let status = child.wait().expect("failed to wait");
    assert_eq!(status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:piped_stdin_stops_execution_after_first_error`

```rust
let mut child = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to run tursodb");

    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(b"select 'one'; select * from missing; select 'two';\n")
        .unwrap();
    drop(stdin);

    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("one"), "first query should execute");
    assert!(
        !stdout.contains("two"),
        "query after error should not execute"
    );
    assert_eq!(output.status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_empty_string_returns_zero`

```rust
let status = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg("")
        .status()
        .expect("failed to run tursodb");

    assert_eq!(status.code(), Some(0));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_returns_exit_code_one_on_query_failure`

```rust
let status = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg("select 'one'; select * from t; select 'two';")
        .status()
        .expect("failed to run tursodb");

    assert_eq!(status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_returns_exit_code_zero_on_success`

```rust
let status = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg("select 'one'; select 'two';")
        .status()
        .expect("failed to run tursodb");

    assert_eq!(status.code(), Some(0));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_runtime_error_returns_nonzero`

```rust
let sql = "create table t(x integer primary key); \
               insert into t values(1); \
               insert into t values(1); \
               select 'after';";
    let status = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg(sql)
        .status()
        .expect("failed to run tursodb");

    assert_eq!(status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_runtime_error_stops_execution`

```rust
let sql = "create table t(x integer primary key); \
               insert into t values(1); \
               insert into t values(1); \
               select 'after';";
    let output = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg(sql)
        .output()
        .expect("failed to run tursodb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("after"),
        "query after runtime error should not execute"
    );
    assert_eq!(output.status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_stops_execution_after_first_error`

```rust
let output = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg("select 'one'; select * from t; select 'two';")
        .output()
        .expect("failed to run tursodb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("one"), "first query should execute");
    assert!(
        !stdout.contains("two"),
        "query after error should not execute"
    );
    assert_eq!(output.status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sql_argument_syntax_error_returns_nonzero`

```rust
let status = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg("select from;")
        .status()
        .expect("failed to run tursodb");

    assert_eq!(status.code(), Some(1));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sqlite_dbpage_update_allows_unsafe_testing`

```rust
let sql = "create table t(x); update sqlite_dbpage set data = data where pgno = 1; select 'after_update';";
    let output = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg("--unsafe-testing")
        .arg(":memory:")
        .arg(sql)
        .output()
        .expect("failed to run tursodb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("after_update"),
        "expected query after update to run"
    );
    assert_eq!(output.status.code(), Some(0));
```
---
## `test:cli/tests/non_interactive_exit_code.rs:sqlite_dbpage_update_requires_unsafe_testing`

```rust
let sql = "create table t(x); update sqlite_dbpage set data = data where pgno = 1; select 'after_update';";
    let output = Command::new(env!("CARGO_BIN_EXE_tursodb"))
        .arg(":memory:")
        .arg(sql)
        .output()
        .expect("failed to run tursodb");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("after_update"),
        "query after sqlite_dbpage update should not execute without unsafe testing"
    );
    assert_eq!(output.status.code(), Some(1));
```
---
## `test:sqlite3/tests/compat/mod.rs:test_close`

```rust
unsafe {
            assert_eq!(sqlite3_close(ptr::null_mut()), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_disable_wal_checkpoint`

```rust
let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            unsafe {
                let mut db = ptr::null_mut();
                let path = temp_file.path();
                let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
                assert_eq!(sqlite3_open(c_path.as_ptr(), &mut db), SQLITE_OK);
                // Create a table and insert a row.
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"CREATE TABLE test (id INTEGER PRIMARY KEY)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"INSERT INTO test (id) VALUES (0)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

                let mut log_size = 0;
                let mut checkpoint_count = 0;

                assert_eq!(
                    sqlite3_wal_checkpoint_v2(
                        db,
                        ptr::null(),
                        SQLITE_CHECKPOINT_PASSIVE,
                        &mut log_size,
                        &mut checkpoint_count
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_callback_abort`

```rust
unsafe {
            // Callback that aborts after first row
            unsafe extern "C" fn abort_callback(
                context: *mut std::ffi::c_void,
                _n_cols: std::ffi::c_int,
                _values: *mut *mut std::ffi::c_char,
                _cols: *mut *mut std::ffi::c_char,
            ) -> std::ffi::c_int {
                let count = &mut *(context as *mut i32);
                *count += 1;
                if *count >= 1 {
                    return 1; // Abort
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_empty_statements`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Multiple semicolons and whitespace should be handled gracefully
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE test(x INTEGER);;;\n\n;\t;INSERT INTO test VALUES(1);;;".as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            // Verify both statements executed
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT x FROM test".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            assert_eq!(sqlite3_column_int(stmt, 0), 1);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_error_stops_execution`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut err_msg = ptr::null_mut();

            // Second statement has error, third should not execute
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE test(x INTEGER);\
              INSERT INTO nonexistent VALUES(1);\
              CREATE TABLE should_not_exist(y INTEGER);"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                &mut err_msg,
            );

            assert_eq!(rc, SQLITE_ERROR);

            // Verify third statement didn't execute
            let mut stmt = ptr::null_mut();
            let check_rc = sqlite3_prepare_v2(
                db,
                c"SELECT name FROM sqlite_master WHERE type='table' AND name='should_not_exist'"
                    .as_ptr(),
                -1,
                &mut stmt,
                ptr::null_mut(),
            );
            assert_eq!(check_rc, SQLITE_OK);
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE); // No rows = table doesn't exist
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            if !err_msg.is_null() {
                sqlite3_free(err_msg as *mut std::ffi::c_void);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_multi_statement_dml`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Multiple DML statements in one exec call
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE bind_text(x TEXT);\
              INSERT INTO bind_text(x) VALUES('TEXT1');\
              INSERT INTO bind_text(x) VALUES('TEXT2');"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            // Verify the data was inserted
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT COUNT(*) FROM bind_text".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            assert_eq!(sqlite3_column_int(stmt, 0), 2);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_multi_statement_mixed_dml_select`

```rust
unsafe {
            // Callback that counts invocations
            unsafe extern "C" fn count_callback(
                context: *mut std::ffi::c_void,
                _n_cols: std::ffi::c_int,
                _values: *mut *mut std::ffi::c_char,
                _cols: *mut *mut std::ffi::c_char,
            ) -> std::ffi::c_int {
                let count = &mut *(context as *mut i32);
                *count += 1;
                0
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_multi_statement_with_escaped_quotes`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Test escaped quotes
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE test_quotes(x TEXT);\
              INSERT INTO test_quotes(x) VALUES('it''s working');\
              INSERT INTO test_quotes(x) VALUES(\"quote\"\"test\"\"\");"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT x FROM test_quotes ORDER BY rowid".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val1 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val1, "it's working");

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val2 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val2, "quote\"test\"");

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_multi_statement_with_semicolons_in_strings`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Semicolons inside strings should not split statements
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE test_semicolon(x TEXT);\
              INSERT INTO test_semicolon(x) VALUES('value;with;semicolons');\
              INSERT INTO test_semicolon(x) VALUES(\"another;value\");"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            // Verify the values contain semicolons
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT x FROM test_semicolon ORDER BY rowid".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val1 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val1, "value;with;semicolons");

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val2 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val2, "another;value");

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_nested_quotes`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Mix of quote types and nesting
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE test(x TEXT);\
              INSERT INTO test VALUES('single \"double\" inside');\
              INSERT INTO test VALUES(\"double 'single' inside\");\
              INSERT INTO test VALUES('mix;\"quote\";types');"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            // Verify values
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT x FROM test ORDER BY rowid".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val1 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val1, "single \"double\" inside");

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val2 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val2, "double 'single' inside");

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let val3 = std::ffi::CStr::from_ptr(sqlite3_column_text(stmt, 0))
                .to_str()
                .unwrap();
            assert_eq!(val3, "mix;\"quote\";types");

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_transaction_rollback`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Test transaction rollback in multi-statement
            let rc = sqlite3_exec(
                db,
                c"CREATE TABLE test(x INTEGER);\
              BEGIN TRANSACTION;\
              INSERT INTO test VALUES(1);\
              INSERT INTO test VALUES(2);\
              ROLLBACK;"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            // Table should exist but be empty due to rollback
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT COUNT(*) FROM test".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            assert_eq!(sqlite3_column_int(stmt, 0), 0); // No rows due to rollback
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_with_comments`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // SQL comments shouldn't affect statement splitting
            let rc = sqlite3_exec(
                db,
                c"-- This is a comment\n\
              CREATE TABLE test(x INTEGER); -- inline comment\n\
              INSERT INTO test VALUES(1); -- semicolon in comment ;\n\
              INSERT INTO test VALUES(2) -- end with comment"
                    .as_ptr(),
                None,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(rc, SQLITE_OK);

            // Verify both inserts worked
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT COUNT(*) FROM test".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            assert_eq!(sqlite3_column_int(stmt, 0), 2);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_with_pragma`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Callback to capture pragma results
            unsafe extern "C" fn pragma_callback(
                context: *mut std::ffi::c_void,
                _n_cols: std::ffi::c_int,
                _values: *mut *mut std::ffi::c_char,
                _cols: *mut *mut std::ffi::c_char,
            ) -> std::ffi::c_int {
                let count = &mut *(context as *mut i32);
                *count += 1;
                0
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_with_returning_clause`

```rust
unsafe {
            // Callback for RETURNING results
            unsafe extern "C" fn exec_callback(
                context: *mut std::ffi::c_void,
                n_cols: std::ffi::c_int,
                values: *mut *mut std::ffi::c_char,
                _cols: *mut *mut std::ffi::c_char,
            ) -> std::ffi::c_int {
                let results = &mut *(context as *mut Vec<Vec<String>>);
                let mut row = Vec::new();
                for i in 0..n_cols as isize {
                    let value_ptr = *values.offset(i);
                    let value = if value_ptr.is_null() {
                        String::from("NULL")
```
---
## `test:sqlite3/tests/compat/mod.rs:test_exec_with_select_callback`

```rust
unsafe {
            // Callback that collects results
            unsafe extern "C" fn exec_callback(
                context: *mut std::ffi::c_void,
                n_cols: std::ffi::c_int,
                values: *mut *mut std::ffi::c_char,
                _cols: *mut *mut std::ffi::c_char,
            ) -> std::ffi::c_int {
                let results = &mut *(context as *mut Vec<Vec<String>>);
                let mut row = Vec::new();

                for i in 0..n_cols as isize {
                    let value_ptr = *values.offset(i);
                    let value = if value_ptr.is_null() {
                        String::from("NULL")
```
---
## `test:sqlite3/tests/compat/mod.rs:test_get_autocommit`

```rust
unsafe {
                let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
                let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
                let mut db = ptr::null_mut();
                assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

                // Should be in autocommit mode by default
                assert_eq!(sqlite3_get_autocommit(db), 1);

                // Begin a transaction
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(db, c"BEGIN".as_ptr(), -1, &mut stmt, ptr::null_mut()),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

                // Should NOT be in autocommit mode during transaction
                assert_eq!(sqlite3_get_autocommit(db), 0);

                // Create a table within the transaction
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"CREATE TABLE test (id INTEGER PRIMARY KEY)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

                // Still not in autocommit mode
                assert_eq!(sqlite3_get_autocommit(db), 0);

                // Commit the transaction
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(db, c"COMMIT".as_ptr(), -1, &mut stmt, ptr::null_mut()),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

                // Should be back in autocommit mode after commit
                assert_eq!(sqlite3_get_autocommit(db), 1);

                // Test with ROLLBACK
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(db, c"BEGIN".as_ptr(), -1, &mut stmt, ptr::null_mut()),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

                assert_eq!(sqlite3_get_autocommit(db), 0);

                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(db, c"ROLLBACK".as_ptr(), -1, &mut stmt, ptr::null_mut()),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

                // Should be back in autocommit mode after rollback
                assert_eq!(sqlite3_get_autocommit(db), 1);

                assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_libversion`

```rust
unsafe {
            let version = sqlite3_libversion();
            assert!(!version.is_null());
```
---
## `test:sqlite3/tests/compat/mod.rs:test_libversion_number`

```rust
unsafe {
            let version_num = sqlite3_libversion_number();
            assert!(version_num >= 3042000);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_open_existing`

```rust
unsafe {
            let mut db = ptr::null_mut();
            assert_eq!(
                sqlite3_open(c"../testing/system/testing_clone.db".as_ptr(), &mut db),
                SQLITE_OK
            );
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_open_not_found`

```rust
unsafe {
            let mut db = ptr::null_mut();
            assert_eq!(
                sqlite3_open(c"not-found/local.db".as_ptr(), &mut db),
                SQLITE_CANTOPEN
            );
```
---
## `test:sqlite3/tests/compat/mod.rs:test_prepare_misuse`

```rust
unsafe {
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(db, c"SELECT 1".as_ptr(), -1, &mut stmt, ptr::null_mut()),
                SQLITE_OK
            );

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_read_frame`

```rust
unsafe {
                let mut db = ptr::null_mut();
                let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
                let path = temp_file.path();
                let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
                assert_eq!(sqlite3_open(c_path.as_ptr(), &mut db), SQLITE_OK);
                // Create a table and insert a row.
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"CREATE TABLE test (id INTEGER PRIMARY KEY)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"INSERT INTO test (id) VALUES (1)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
                // Check that WAL has three frames.
                let mut frame_count = 0;
                assert_eq!(libsql_wal_frame_count(db, &mut frame_count), SQLITE_OK);
                assert_eq!(frame_count, 3);
                for i in 1..frame_count + 1 {
                    let frame_len = 4096 + 24;
                    let mut frame = vec![0; frame_len];
                    assert_eq!(
                        libsql_wal_get_frame(db, i, frame.as_mut_ptr(), frame_len as u32),
                        SQLITE_OK
                    );
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_bind_blob`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_bind_blob_rs (id INTEGER PRIMARY KEY, data BLOB)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_bind_blob_rs (data) VALUES (?)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            let data1 = b"\x01\x02\x03\x04\x05";
            assert_eq!(
                sqlite3_bind_blob(
                    stmt,
                    1,
                    data1.as_ptr() as *const _,
                    data1.len() as i32,
                    None
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_bind_blob_rs (data) VALUES (?)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            let data2 = b"\xAA\xBB\xCC\xDD";
            assert_eq!(
                sqlite3_bind_blob(stmt, 1, data2.as_ptr() as *const _, 2, None),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT data FROM test_bind_blob_rs ORDER BY id".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let col1_ptr = sqlite3_column_blob(stmt, 0);
            let col1_len = sqlite3_column_bytes(stmt, 0);
            let col1_slice = std::slice::from_raw_parts(col1_ptr as *const u8, col1_len as usize);
            assert_eq!(col1_slice, data1);

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let col2_ptr = sqlite3_column_blob(stmt, 0);
            let col2_len = sqlite3_column_bytes(stmt, 0);
            let col2_slice = std::slice::from_raw_parts(col2_ptr as *const u8, col2_len as usize);
            assert_eq!(col2_slice, &data2[..2]);

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_bind_int`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_bind (id INTEGER PRIMARY KEY, value INTEGER)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_bind (value) VALUES (?)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_bind_int(stmt, 1, 42), SQLITE_OK);
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT value FROM test_bind LIMIT 1".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            assert_eq!(sqlite3_column_int(stmt, 0), 42);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_bind_parameter_index`

```rust
const SQLITE_OK: i32 = 0;

        unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            let mut stmt: *mut sqlite3_stmt = ptr::null_mut();

            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT * FROM sqlite_master WHERE name = :table_name AND type = :object_type"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut()
                ),
                SQLITE_OK
            );

            let index1 = sqlite3_bind_parameter_index(stmt, c":table_name".as_ptr());
            assert_eq!(index1, 1);

            let index2 = sqlite3_bind_parameter_index(stmt, c":object_type".as_ptr());
            assert_eq!(index2, 2);

            let index3 = sqlite3_bind_parameter_index(stmt, c":nonexistent".as_ptr());
            assert_eq!(index3, 0);

            let index4 = sqlite3_bind_parameter_index(stmt, ptr::null());
            assert_eq!(index4, 0);

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_bind_parameter_name_and_count`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_params (id INTEGER PRIMARY KEY, value TEXT)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_params (id, value) VALUES (?1, ?2)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            let param_count = sqlite3_bind_parameter_count(stmt);
            assert_eq!(param_count, 2);

            println!("parameter count {param_count
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_bind_text`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_bind_text_rs (id INTEGER PRIMARY KEY, value TEXT)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            let destructor = std::mem::transmute::<
                isize,
                Option<unsafe extern "C" fn(*mut std::ffi::c_void)>,
            >(-1isize);
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_bind_text_rs (value) VALUES (?)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            let val = std::ffi::CString::new("hello world").unwrap();
            assert_eq!(
                sqlite3_bind_text(stmt, 1, val.as_ptr(), -1, destructor),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_bind_text_rs (value) VALUES (?)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            let val2 = std::ffi::CString::new("abcdef").unwrap();
            assert_eq!(
                sqlite3_bind_text(stmt, 1, val2.as_ptr(), 3, destructor),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT value FROM test_bind_text_rs ORDER BY id".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            let col1_ptr = sqlite3_column_text(stmt, 0);
            assert!(!col1_ptr.is_null());
            let col1_str = std::ffi::CStr::from_ptr(col1_ptr).to_str().unwrap();
            assert_eq!(col1_str, "hello world");

            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);

            let col2_ptr = sqlite3_column_text(stmt, 0);
            let col2_len = sqlite3_column_bytes(stmt, 0);
            assert!(!col2_ptr.is_null());

            let col2_slice = std::slice::from_raw_parts(col2_ptr as *const u8, col2_len as usize);
            let col2_str = std::str::from_utf8(col2_slice).unwrap().to_owned();

            assert_eq!(col2_str, "abc");
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_busy_handler`

```rust
unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            // Test setting a custom busy handler with context
            let mut max_retries: i32 = 3;
            assert_eq!(
                sqlite3_busy_handler(
                    db,
                    Some(busy_handler_retry_n),
                    &mut max_retries as *mut i32 as *mut libc::c_void
                ),
                SQLITE_OK
            );

            // Test clearing the busy handler by passing NULL callback
            assert_eq!(sqlite3_busy_handler(db, None, ptr::null_mut()), SQLITE_OK);

            // Test setting busy handler that never retries
            assert_eq!(
                sqlite3_busy_handler(db, Some(busy_handler_never_retry), ptr::null_mut()),
                SQLITE_OK
            );

            // Test setting busy handler that always retries
            assert_eq!(
                sqlite3_busy_handler(db, Some(busy_handler_always_retry), ptr::null_mut()),
                SQLITE_OK
            );

            // Test that busy_timeout clears a previously set busy_handler
            assert_eq!(
                sqlite3_busy_handler(
                    db,
                    Some(busy_handler_retry_n),
                    &mut max_retries as *mut i32 as *mut libc::c_void
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_busy_timeout(db, 500), SQLITE_OK);

            // Test that busy_handler clears a previously set busy_timeout
            assert_eq!(sqlite3_busy_timeout(db, 1000), SQLITE_OK);
            assert_eq!(
                sqlite3_busy_handler(
                    db,
                    Some(busy_handler_retry_n),
                    &mut max_retries as *mut i32 as *mut libc::c_void
                ),
                SQLITE_OK
            );

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_busy_timeout`

```rust
unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            // Test setting a positive timeout
            assert_eq!(sqlite3_busy_timeout(db, 1000), SQLITE_OK);

            // Test setting a zero timeout (disables busy handler)
            assert_eq!(sqlite3_busy_timeout(db, 0), SQLITE_OK);

            // Test setting a negative timeout (also disables busy handler)
            assert_eq!(sqlite3_busy_timeout(db, -1), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_changes`

```rust
unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            // // Initially no changes
            assert_eq!(sqlite3_changes(db), 0);
            assert_eq!(sqlite3_changes64(db), 0);

            // Create a table
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_changes (id INTEGER PRIMARY KEY, value TEXT)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            // Still no changes after CREATE TABLE
            assert_eq!(sqlite3_changes(db), 0);
            assert_eq!(sqlite3_changes64(db), 0);

            // Insert a single row
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_changes (value) VALUES ('test1')".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            // Should have 1 change
            assert_eq!(sqlite3_changes(db), 1);
            assert_eq!(sqlite3_changes64(db), 1);

            // Insert multiple rows
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_changes (value) VALUES ('test2'), ('test3'), ('test4')"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            // Should have 3 changes
            assert_eq!(sqlite3_changes(db), 3);
            assert_eq!(sqlite3_changes64(db), 3);

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_clear_bindings`

```rust
unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            let mut stmt: *mut sqlite3_stmt = ptr::null_mut();

            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE person (id INTEGER, name TEXT, age INTEGER)".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut()
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO person (id, name, age) VALUES (1, 'John', 25), (2, 'Jane', 30)"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut()
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT * FROM person WHERE id = ? AND age > ?".as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut()
                ),
                SQLITE_OK
            );

            // Bind parameters - should find John (id=1, age=25 > 20)
            assert_eq!(sqlite3_bind_int(stmt, 1, 1), SQLITE_OK);
            assert_eq!(sqlite3_bind_int(stmt, 2, 20), SQLITE_OK);
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);
            assert_eq!(sqlite3_column_int(stmt, 0), 1);
            assert_eq!(sqlite3_column_int(stmt, 2), 25);

            // Reset and clear bindings, query should return no rows
            assert_eq!(sqlite3_reset(stmt), SQLITE_OK);
            assert_eq!(sqlite3_clear_bindings(stmt), SQLITE_OK);
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_column_decltype`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = std::ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
            sqlite3_prepare_v2(
                db,
                c"CREATE TABLE test_decltype (col_int INTEGER, col_float REAL, col_text TEXT, col_blob BLOB, col_null NULL)".as_ptr(),
                -1,
                &mut stmt,
                std::ptr::null_mut(),
            ),
            SQLITE_OK
        );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT col_int, col_float, col_text, col_blob, col_null FROM test_decltype"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );

            let expected = [
                Some("INTEGER"),
                Some("REAL"),
                Some("TEXT"),
                Some("BLOB"),
                None,
            ];

            for i in 0..sqlite3_column_count(stmt) {
                let decl = sqlite3_column_decltype(stmt, i);

                if decl.is_null() {
                    assert!(expected[i as usize].is_none());
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_column_name`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = std::ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_cols (id INTEGER PRIMARY KEY, value TEXT)".as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT id, value FROM test_cols".as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );

            let col_count = sqlite3_column_count(stmt);
            assert_eq!(col_count, 2);

            let name1 = sqlite3_column_name(stmt, 0);
            assert!(!name1.is_null());
            let name1_str = std::ffi::CStr::from_ptr(name1).to_str().unwrap();
            assert_eq!(name1_str, "id");

            let table_name1 = sqlite3_column_table_name(stmt, 0);
            assert!(!table_name1.is_null());
            let table_name1_str = std::ffi::CStr::from_ptr(table_name1).to_str().unwrap();
            assert_eq!(table_name1_str, "test_cols");

            let name2 = sqlite3_column_name(stmt, 1);
            assert!(!name2.is_null());
            let name2_str = std::ffi::CStr::from_ptr(name2).to_str().unwrap();
            assert_eq!(name2_str, "value");

            // will lead to panic
            //let invalid = sqlite3_column_name(stmt, 5);
            //assert!(invalid.is_null());

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_column_type`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = std::ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
            sqlite3_prepare_v2(
                db,
                c"CREATE TABLE test_types (col_int INTEGER, col_float REAL, col_text TEXT, col_blob BLOB, col_null text)".as_ptr(),
                -1,
                &mut stmt,
                std::ptr::null_mut(),
            ),
            SQLITE_OK
        );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_types VALUES (123, 45.67, 'hello', x'010203', null)"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"SELECT col_int, col_float, col_text, col_blob, col_null FROM test_types"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_ROW);

            assert_eq!(sqlite3_column_type(stmt, 0), SQLITE_INTEGER);
            assert_eq!(sqlite3_column_type(stmt, 1), SQLITE_FLOAT);
            assert_eq!(sqlite3_column_type(stmt, 2), SQLITE_TEXT);
            assert_eq!(sqlite3_column_type(stmt, 3), SQLITE_BLOB);
            assert_eq!(sqlite3_column_type(stmt, 4), SQLITE_NULL);

            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_db_filename`

```rust
const SQLITE_OK: i32 = 0;

        unsafe {
            // Test with in-memory database
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);
            let filename = sqlite3_db_filename(db, c"main".as_ptr());
            assert!(!filename.is_null());
            let filename_str = std::ffi::CStr::from_ptr(filename).to_str().unwrap();
            assert_eq!(filename_str, "");
            assert_eq!(sqlite3_close(db), SQLITE_OK);

            // Open a file-backed database
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            // Test with "main" database name
            let filename = sqlite3_db_filename(db, c"main".as_ptr());
            assert!(!filename.is_null());
            let filename_pathbuf =
                std::fs::canonicalize(std::ffi::CStr::from_ptr(filename).to_str().unwrap())
                    .unwrap();
            assert_eq!(filename_pathbuf, temp_file.path().canonicalize().unwrap());

            // Test with NULL database name (defaults to main)
            let filename_default = sqlite3_db_filename(db, ptr::null());
            assert!(!filename_default.is_null());
            assert_eq!(filename, filename_default);

            // Test with non-existent database name
            let filename = sqlite3_db_filename(db, c"temp".as_ptr());
            assert!(filename.is_null());

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_free_table_null`

```rust
unsafe {
            // Passing null should not crash
            sqlite3_free_table(ptr::null_mut());
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_get_table`

```rust
unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            // Create and populate a table
            assert_eq!(
                sqlite3_exec(
                    db,
                    c"CREATE TABLE t1(id INTEGER, name TEXT)".as_ptr(),
                    None,
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(
                sqlite3_exec(
                    db,
                    c"INSERT INTO t1 VALUES(1, 'alice')".as_ptr(),
                    None,
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(
                sqlite3_exec(
                    db,
                    c"INSERT INTO t1 VALUES(2, 'bob')".as_ptr(),
                    None,
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            // Query via sqlite3_get_table
            let mut result: *mut *mut libc::c_char = ptr::null_mut();
            let mut n_row: libc::c_int = 0;
            let mut n_col: libc::c_int = 0;
            let mut err_msg: *mut libc::c_char = ptr::null_mut();
            assert_eq!(
                sqlite3_get_table(
                    db,
                    c"SELECT id, name FROM t1 ORDER BY id".as_ptr(),
                    &mut result,
                    &mut n_row,
                    &mut n_col,
                    &mut err_msg,
                ),
                SQLITE_OK
            );

            assert_eq!(n_row, 2);
            assert_eq!(n_col, 2);

            // result layout: [col0_name, col1_name, row0_val0, row0_val1, row1_val0, row1_val1]
            let col0 = std::ffi::CStr::from_ptr(*result.add(0));
            let col1 = std::ffi::CStr::from_ptr(*result.add(1));
            assert_eq!(col0.to_str().unwrap(), "id");
            assert_eq!(col1.to_str().unwrap(), "name");

            let r0v0 = std::ffi::CStr::from_ptr(*result.add(2));
            let r0v1 = std::ffi::CStr::from_ptr(*result.add(3));
            assert_eq!(r0v0.to_str().unwrap(), "1");
            assert_eq!(r0v1.to_str().unwrap(), "alice");

            let r1v0 = std::ffi::CStr::from_ptr(*result.add(4));
            let r1v1 = std::ffi::CStr::from_ptr(*result.add(5));
            assert_eq!(r1v0.to_str().unwrap(), "2");
            assert_eq!(r1v1.to_str().unwrap(), "bob");

            sqlite3_free_table(result);

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_get_table_empty_result`

```rust
unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            assert_eq!(
                sqlite3_exec(
                    db,
                    c"CREATE TABLE t1(id INTEGER)".as_ptr(),
                    None,
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            let mut result: *mut *mut libc::c_char = ptr::null_mut();
            let mut n_row: libc::c_int = 0;
            let mut n_col: libc::c_int = 0;
            assert_eq!(
                sqlite3_get_table(
                    db,
                    c"SELECT id FROM t1".as_ptr(),
                    &mut result,
                    &mut n_row,
                    &mut n_col,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );

            assert_eq!(n_row, 0);
            assert_eq!(n_col, 0);

            sqlite3_free_table(result);
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_last_insert_rowid`

```rust
unsafe {
            let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
            let path = std::ffi::CString::new(temp_file.path().to_str().unwrap()).unwrap();
            let mut db = std::ptr::null_mut();
            assert_eq!(sqlite3_open(path.as_ptr(), &mut db), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_rowid (value INTEGER)".as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let mut stmt = std::ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"INSERT INTO test_rowid (value) VALUES (6)".as_ptr(),
                    -1,
                    &mut stmt,
                    std::ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            let last_rowid = sqlite3_last_insert_rowid(db);
            assert!(last_rowid > 0);
            println!("last insert rowid: {last_rowid
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_next_stmt`

```rust
const SQLITE_OK: i32 = 0;

        unsafe {
            let mut db: *mut sqlite3 = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            // Initially, there should be no prepared statements
            let iter = sqlite3_next_stmt(db, ptr::null_mut());
            assert!(iter.is_null());

            // Prepare first statement
            let mut stmt1: *mut sqlite3_stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(db, c"SELECT 1;".as_ptr(), -1, &mut stmt1, ptr::null_mut()),
                SQLITE_OK
            );
            assert!(!stmt1.is_null());

            // Now there should be one statement
            let iter = sqlite3_next_stmt(db, ptr::null_mut());
            assert_eq!(iter, stmt1);

            // And no more after that
            let iter = sqlite3_next_stmt(db, stmt1);
            assert!(iter.is_null());

            // Prepare second statement
            let mut stmt2: *mut sqlite3_stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(db, c"SELECT 2;".as_ptr(), -1, &mut stmt2, ptr::null_mut()),
                SQLITE_OK
            );
            assert!(!stmt2.is_null());

            // Prepare third statement
            let mut stmt3: *mut sqlite3_stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(db, c"SELECT 3;".as_ptr(), -1, &mut stmt3, ptr::null_mut()),
                SQLITE_OK
            );
            assert!(!stmt3.is_null());

            // Count all statements
            let mut count = 0;
            let mut iter = sqlite3_next_stmt(db, ptr::null_mut());
            while !iter.is_null() {
                count += 1;
                iter = sqlite3_next_stmt(db, iter);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_table_column_metadata`

```rust
unsafe {
            let mut db = ptr::null_mut();
            assert_eq!(sqlite3_open(c":memory:".as_ptr(), &mut db), SQLITE_OK);

            // Create a test table
            let mut stmt = ptr::null_mut();
            assert_eq!(
                sqlite3_prepare_v2(
                    db,
                    c"CREATE TABLE test_metadata (id INTEGER PRIMARY KEY, name TEXT NOT NULL, value REAL)"
                        .as_ptr(),
                    -1,
                    &mut stmt,
                    ptr::null_mut(),
                ),
                SQLITE_OK
            );
            assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
            assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);

            // Test column metadata for 'id' column
            let mut data_type: *const libc::c_char = ptr::null();
            let mut coll_seq: *const libc::c_char = ptr::null();
            let mut not_null: libc::c_int = 0;
            let mut primary_key: libc::c_int = 0;
            let mut autoinc: libc::c_int = 0;

            assert_eq!(
                sqlite3_table_column_metadata(
                    db,
                    ptr::null(), // main database
                    c"test_metadata".as_ptr(),
                    c"id".as_ptr(),
                    &mut data_type,
                    &mut coll_seq,
                    &mut not_null,
                    &mut primary_key,
                    &mut autoinc,
                ),
                SQLITE_OK
            );

            // Verify the results
            assert!(!data_type.is_null());
            assert!(!coll_seq.is_null());
            assert_eq!(primary_key, 1); // id is primary key
            assert_eq!(not_null, 0); // INTEGER columns don't have NOT NULL by default
            assert_eq!(autoinc, 0); // not auto-increment

            // Test column metadata for 'name' column
            let mut data_type2: *const libc::c_char = ptr::null();
            let mut coll_seq2: *const libc::c_char = ptr::null();
            let mut not_null2: libc::c_int = 0;
            let mut primary_key2: libc::c_int = 0;
            let mut autoinc2: libc::c_int = 0;

            assert_eq!(
                sqlite3_table_column_metadata(
                    db,
                    ptr::null(), // main database
                    c"test_metadata".as_ptr(),
                    c"name".as_ptr(),
                    &mut data_type2,
                    &mut coll_seq2,
                    &mut not_null2,
                    &mut primary_key2,
                    &mut autoinc2,
                ),
                SQLITE_OK
            );

            // Verify the results
            assert!(!data_type2.is_null());
            assert!(!coll_seq2.is_null());
            assert_eq!(primary_key2, 0); // name is not primary key
            assert_eq!(not_null2, 1); // name has NOT NULL constraint
            assert_eq!(autoinc2, 0); // not auto-increment

            // Test non-existent column
            let mut data_type3: *const libc::c_char = ptr::null();
            let mut coll_seq3: *const libc::c_char = ptr::null();
            let mut not_null3: libc::c_int = 0;
            let mut primary_key3: libc::c_int = 0;
            let mut autoinc3: libc::c_int = 0;

            assert_eq!(
                sqlite3_table_column_metadata(
                    db,
                    ptr::null(), // main database
                    c"test_metadata".as_ptr(),
                    c"nonexistent".as_ptr(),
                    &mut data_type3,
                    &mut coll_seq3,
                    &mut not_null3,
                    &mut primary_key3,
                    &mut autoinc3,
                ),
                SQLITE_ERROR
            );

            // Test non-existent table
            let mut data_type4: *const libc::c_char = ptr::null();
            let mut coll_seq4: *const libc::c_char = ptr::null();
            let mut not_null4: libc::c_int = 0;
            let mut primary_key4: libc::c_int = 0;
            let mut autoinc4: libc::c_int = 0;

            assert_eq!(
                sqlite3_table_column_metadata(
                    db,
                    ptr::null(), // main database
                    c"nonexistent_table".as_ptr(),
                    c"id".as_ptr(),
                    &mut data_type4,
                    &mut coll_seq4,
                    &mut not_null4,
                    &mut primary_key4,
                    &mut autoinc4,
                ),
                SQLITE_ERROR
            );

            // Test rowid column
            let mut data_type5: *const libc::c_char = ptr::null();
            let mut coll_seq5: *const libc::c_char = ptr::null();
            let mut not_null5: libc::c_int = 0;
            let mut primary_key5: libc::c_int = 0;
            let mut autoinc5: libc::c_int = 0;

            assert_eq!(
                sqlite3_table_column_metadata(
                    db,
                    ptr::null(), // main database
                    c"test_metadata".as_ptr(),
                    c"rowid".as_ptr(),
                    &mut data_type5,
                    &mut coll_seq5,
                    &mut not_null5,
                    &mut primary_key5,
                    &mut autoinc5,
                ),
                SQLITE_OK
            );

            // Verify rowid results
            assert!(!data_type5.is_null());
            assert!(!coll_seq5.is_null());
            assert_eq!(primary_key5, 1); // rowid is primary key
            assert_eq!(not_null5, 0);
            assert_eq!(autoinc5, 0);

            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_wal_frame_count`

```rust
unsafe {
                let temp_file = tempfile::NamedTempFile::with_suffix(".db").unwrap();
                let path = temp_file.path();
                let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
                let mut db = ptr::null_mut();
                assert_eq!(sqlite3_open(c_path.as_ptr(), &mut db), SQLITE_OK);
                // Ensure that WAL is initially empty.
                let mut frame_count = 0;
                assert_eq!(libsql_wal_frame_count(db, &mut frame_count), SQLITE_OK);
                assert_eq!(frame_count, 0);
                // Create a table and insert a row.
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"CREATE TABLE test (id INTEGER PRIMARY KEY)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
                let mut stmt = ptr::null_mut();
                assert_eq!(
                    sqlite3_prepare_v2(
                        db,
                        c"INSERT INTO test (id) VALUES (1)".as_ptr(),
                        -1,
                        &mut stmt,
                        ptr::null_mut()
                    ),
                    SQLITE_OK
                );
                assert_eq!(sqlite3_step(stmt), SQLITE_DONE);
                assert_eq!(sqlite3_finalize(stmt), SQLITE_OK);
                // Check that WAL has three frames.
                assert_eq!(libsql_wal_frame_count(db, &mut frame_count), SQLITE_OK);
                assert_eq!(frame_count, 3);
                assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:tests/fuzz/custom_types.rs:fuzz_custom_type_invariants`

```rust
maybe_setup_tracing();
        let (mut rng, seed) = rng_from_time_or_env();
        println!("fuzz_custom_type_invariants seed: {seed
```
---
## `test:tests/fuzz/journal_mode.rs:journal_mode_delete_lost_on_switch`

```rust
maybe_setup_tracing();

    let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create schema
    let schema = "CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT);";

    // Open and enable MVCC via PRAGMA
    let limbo_db = TempDatabaseBuilder::new().with_db_path(&db_path).build();
    let conn = limbo_db.connect_limbo();
    conn.pragma_update("journal_mode", "'mvcc'")
        .expect("enable mvcc");

    // Create table
    conn.prepare_execute_batch(schema).unwrap();

    // Verify we start in mvcc mode
    let mode = get_limbo_journal_mode(&conn);
    println!("Initial mode: {mode
```
---
## `test:tests/fuzz/journal_mode.rs:journal_mode_update_then_delete_btree_resident`

```rust
maybe_setup_tracing();

    let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    let schema = "CREATE TABLE t(id INTEGER PRIMARY KEY, val TEXT);";

    // Open and enable MVCC via PRAGMA
    let limbo_db = TempDatabaseBuilder::new().with_db_path(&db_path).build();
    let conn = limbo_db.connect_limbo();
    conn.pragma_update("journal_mode", "'mvcc'")
        .expect("enable mvcc");

    // Create table
    conn.prepare_execute_batch(schema).unwrap();

    // Step 1: Insert a row in MVCC mode
    conn.execute("INSERT INTO t(id, val) VALUES (1, 'original')")
        .unwrap();
    println!("Step 1: Inserted row in MVCC");

    // Step 2: Switch to WAL (checkpoints the row to B-tree)
    let result = conn
        .pragma_update("journal_mode", "'wal'")
        .expect("switch to wal");
    println!("Step 2: Switched to WAL (checkpointed to B-tree)");
    assert_eq!(result[0][0].to_string(), "wal");

    // Verify row exists in B-tree
    let rows: Vec<(i64, String)> = conn.exec_rows("SELECT * FROM t ORDER BY id");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "original");

    // Step 3: Switch back to MVCC (creates new MvStore, row only in B-tree)
    // This is the key step - after this, the row ONLY exists in B-tree,
    // and durable_txid_max will be 0 (None when converted to NonZeroU64)
    let result = conn
        .pragma_update("journal_mode", "'mvcc'")
        .expect("switch to mvcc");
    println!("Step 3: Switched to MVCC (new MvStore, row only in B-tree)");
    assert_eq!(result[0][0].to_string(), "mvcc");

    // Verify row still visible (reading from B-tree)
    let rows: Vec<(i64, String)> = conn.exec_rows("SELECT * FROM t ORDER BY id");
    assert_eq!(rows.len(), 1);

    // Step 4: UPDATE the row
    // This creates an MVCC version for a B-tree-resident row.
    // Without the btree_resident fix, this version has btree_resident=false
    // and begin_ts > 0, which means checkpoint won't recognize it as a B-tree row.
    conn.execute("UPDATE t SET val = 'updated' WHERE id = 1")
        .unwrap();
    println!("Step 4: Updated row (creates MVCC version for B-tree resident row)");

    // Verify update worked
    let rows: Vec<(i64, String)> = conn.exec_rows("SELECT * FROM t ORDER BY id");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].1, "updated");

    // Step 5: DELETE the row
    conn.execute("DELETE FROM t WHERE id = 1").unwrap();
    println!("Step 5: Deleted row");

    // Verify delete worked in MVCC
    let rows: Vec<(i64, String)> = conn.exec_rows("SELECT * FROM t ORDER BY id");
    assert_eq!(rows.len(), 0, "Row should be deleted in MVCC");

    // Step 6: Switch to WAL - this triggers checkpoint
    // BUG: Without btree_resident fix, the checkpoint doesn't recognize
    // that the deleted row existed in B-tree, so it doesn't checkpoint the delete.
    // Result: the old value from B-tree "reappears"
    let result = conn
        .pragma_update("journal_mode", "'wal'")
        .expect("switch to wal");
    println!("Step 6: Switched to WAL (checkpoint)");
    assert_eq!(result[0][0].to_string(), "wal");

    // BUG CHECK: Row should remain deleted after checkpoint
    // Without the fix, the row reappears with value "original" (from B-tree)
    let rows: Vec<(i64, String)> = conn.exec_rows("SELECT * FROM t ORDER BY id");
    println!("Final state: {rows:?
```
---
## `test:tests/fuzz/test_join_optimizer.rs:test_star_schema_fuzz`

```rust
let (mut rng, seed) = rng_from_time_or_env();
    println!("seed: {seed
```
---
## `test:tests/integration/assert_details.rs:test_turso_assert_details_in_panic_message`

```rust
let msg = panic_message(|| {
            let page_id = 42;
            turso_macros::turso_assert!(false, "page must be dirty", { "page_id": page_id
```
---
## `test:tests/integration/assert_details.rs:test_turso_assert_eq_details_in_panic_message`

```rust
let msg = panic_message(|| {
            let expected = 10;
            turso_macros::turso_assert_eq!(1, 2, "values must match", { "expected": expected
```
---
## `test:tests/integration/assert_details.rs:test_turso_assert_greater_than_details`

```rust
let msg = panic_message(|| {
            let limit = 100;
            turso_macros::turso_assert_greater_than!(5, 10, "must be greater", { "limit": limit
```
---
## `test:tests/integration/assert_details.rs:test_turso_assert_multiple_details`

```rust
let msg = panic_message(|| {
            let x = 1;
            let y = 2;
            turso_macros::turso_assert!(false, "check failed", { "x": x, "y": y
```
---
## `test:tests/integration/assert_details.rs:test_turso_assert_no_details_still_works`

```rust
let msg = panic_message(|| {
            turso_macros::turso_assert!(false, "simple message");
```
---
## `test:tests/integration/assert_details.rs:test_turso_assert_string_detail_values`

```rust
let msg = panic_message(|| {
            let state = format!("{:?
```
---
## `test:tests/integration/custom_types.rs:test_custom_types_persist_across_reopen`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_reopen.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);

        // First session: create a custom type, table, and insert data
        {
            let db = TempDatabase::new_with_existent_with_opts(&path, opts);
            let conn = db.connect_limbo();
            conn.execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
                .unwrap();
            conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
                .unwrap();
            conn.execute("INSERT INTO t1 VALUES (1, 42)").unwrap();
            conn.execute("INSERT INTO t1 VALUES (2, 100)").unwrap();

            // Sanity check: values are decoded in the first session
            let rows: Vec<(i64, i64)> = conn.exec_rows("SELECT id, amount FROM t1 ORDER BY id");
            assert_eq!(rows, vec![(1, 42), (2, 100)]);
            conn.close().unwrap();
```
---
## `test:tests/integration/custom_types.rs:test_custom_types_survive_schema_change_after_reopen`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_schema_change.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);

        // First session: create type, table, insert data
        {
            let db = TempDatabase::new_with_existent_with_opts(&path, opts);
            let conn = db.connect_limbo();
            conn.execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
                .unwrap();
            conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
                .unwrap();
            conn.execute("INSERT INTO t1 VALUES (1, 42)").unwrap();
            conn.close().unwrap();
```
---
## `test:tests/integration/custom_types.rs:test_multi_row_update_does_not_double_encode`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_multi_update.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);
        let db = TempDatabase::new_with_existent_with_opts(&path, opts);
        let conn = db.connect_limbo();

        conn.execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
            .unwrap();
        conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
            .unwrap();
        conn.execute("INSERT INTO t1 VALUES (1, 10)").unwrap();
        conn.execute("INSERT INTO t1 VALUES (2, 20)").unwrap();
        conn.execute("INSERT INTO t1 VALUES (3, 30)").unwrap();
        conn.execute("INSERT INTO t1 VALUES (4, 40)").unwrap();
        conn.execute("INSERT INTO t1 VALUES (5, 50)").unwrap();

        // UPDATE all rows with a constant value
        conn.execute("UPDATE t1 SET amount = 99").unwrap();
        let rows: Vec<(i64, i64)> = conn.exec_rows("SELECT id, amount FROM t1 ORDER BY id");
        assert_eq!(
            rows,
            vec![(1, 99), (2, 99), (3, 99), (4, 99), (5, 99)],
            "All rows must have amount=99 after UPDATE, not progressively double-encoded values"
        );

        // UPDATE with WHERE matching multiple rows
        conn.execute("UPDATE t1 SET amount = 42 WHERE id > 2")
            .unwrap();
        let rows: Vec<(i64, i64)> = conn.exec_rows("SELECT id, amount FROM t1 ORDER BY id");
        assert_eq!(
            rows,
            vec![(1, 99), (2, 99), (3, 42), (4, 42), (5, 42)],
            "WHERE-filtered multi-row UPDATE must encode each row exactly once"
        );

        // Multi-column UPDATE with different custom types
        conn.execute("CREATE TYPE score BASE integer ENCODE value * 10 DECODE value / 10")
            .unwrap();
        conn.execute("CREATE TABLE t2(id INTEGER PRIMARY KEY, a cents, b score) STRICT")
            .unwrap();
        conn.execute("INSERT INTO t2 VALUES (1, 10, 5)").unwrap();
        conn.execute("INSERT INTO t2 VALUES (2, 20, 6)").unwrap();
        conn.execute("INSERT INTO t2 VALUES (3, 30, 7)").unwrap();

        conn.execute("UPDATE t2 SET a = 50, b = 8").unwrap();
        let rows: Vec<(i64, i64, i64)> = conn.exec_rows("SELECT id, a, b FROM t2 ORDER BY id");
        assert_eq!(
            rows,
            vec![(1, 50, 8), (2, 50, 8), (3, 50, 8)],
            "Multi-column UPDATE must encode each column independently and correctly"
        );
```
---
## `test:tests/integration/custom_types.rs:test_new_connection_sees_custom_types`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_new_conn.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);

        let db = TempDatabase::new_with_existent_with_opts(&path, opts);
        let conn1 = db.connect_limbo();
        conn1
            .execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
            .unwrap();
        conn1
            .execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
            .unwrap();
        conn1.execute("INSERT INTO t1 VALUES (1, 42)").unwrap();

        // Second connection on the same database
        let conn2 = db.connect_limbo();
        let rows: Vec<(i64,)> = conn2.exec_rows("SELECT amount FROM t1 WHERE id = 1");
        assert_eq!(
            rows,
            vec![(42,)],
            "New connection should decode custom type values"
        );

        // Second connection should also be able to insert with encoding
        conn2.execute("INSERT INTO t1 VALUES (2, 77)").unwrap();
        let rows: Vec<(i64,)> = conn2.exec_rows("SELECT amount FROM t1 WHERE id = 2");
        assert_eq!(rows, vec![(77,)]);
```
---
## `test:tests/integration/custom_types.rs:test_self_join_on_custom_type_column`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_self_join.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);
        let db = TempDatabase::new_with_existent_with_opts(&path, opts);
        let conn = db.connect_limbo();

        conn.execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
            .unwrap();
        conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
            .unwrap();
        conn.execute("INSERT INTO t1 VALUES (1, 10)").unwrap();
        conn.execute("INSERT INTO t1 VALUES (2, 20)").unwrap();
        conn.execute("INSERT INTO t1 VALUES (3, 10)").unwrap();

        // Self-join: rows with equal decoded amounts must match
        let mut rows: Vec<(i64, i64)> =
            conn.exec_rows("SELECT a.id, b.id FROM t1 a, t1 b WHERE a.amount = b.amount");
        rows.sort();
        assert_eq!(
            rows,
            vec![(1, 1), (1, 3), (2, 2), (3, 1), (3, 3)],
            "Self-join on custom type column should return matching rows"
        );

        // LEFT JOIN variant: unmatched rows should produce NULLs
        conn.execute("CREATE TABLE t2(id INTEGER PRIMARY KEY, amount cents) STRICT")
            .unwrap();
        conn.execute("INSERT INTO t2 VALUES (1, 10)").unwrap();
        conn.execute("INSERT INTO t2 VALUES (2, 20)").unwrap();

        let rows: Vec<(i64, String)> = conn.exec_rows(
            "SELECT t1.id, COALESCE(CAST(t2.id AS TEXT), 'NULL') \
             FROM t1 LEFT JOIN t2 ON t1.amount = t2.amount ORDER BY t1.id",
        );
        assert_eq!(
            rows,
            vec![
                (1, "1".to_string()),
                (2, "2".to_string()),
                (3, "1".to_string()),
            ],
            "LEFT JOIN on custom type column should find matches and produce NULLs for non-matches"
        );
```
---
## `test:tests/integration/custom_types.rs:test_upsert_does_not_double_encode_custom_types`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_upsert.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);
        let db = TempDatabase::new_with_existent_with_opts(&path, opts);
        let conn = db.connect_limbo();

        conn.execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
            .unwrap();
        conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
            .unwrap();
        conn.execute("INSERT INTO t1 VALUES (1, 42)").unwrap();

        // Bug 7: excluded.amount should not be double-encoded
        conn.execute(
            "INSERT INTO t1 VALUES (1, 50) ON CONFLICT(id) DO UPDATE SET amount = excluded.amount",
        )
        .unwrap();
        let rows: Vec<(i64,)> = conn.exec_rows("SELECT amount FROM t1 WHERE id = 1");
        assert_eq!(
            rows,
            vec![(50,)],
            "UPSERT with excluded.amount should produce 50, not double-encoded value"
        );

        // Bug 15: sequential UPSERTs must not progressively corrupt data
        conn.execute(
            "INSERT INTO t1 VALUES (1, 75) ON CONFLICT(id) DO UPDATE SET amount = excluded.amount",
        )
        .unwrap();
        let rows: Vec<(i64,)> = conn.exec_rows("SELECT amount FROM t1 WHERE id = 1");
        assert_eq!(
            rows,
            vec![(75,)],
            "Sequential UPSERT should produce 75, not progressively corrupted value"
        );

        // Bug 13: WHERE clause in DO UPDATE must see decoded values
        conn.execute("INSERT INTO t1 VALUES (2, 10)").unwrap();
        conn.execute(
            "INSERT INTO t1 VALUES (2, 99) ON CONFLICT(id) DO UPDATE SET amount = excluded.amount WHERE t1.amount < 20",
        )
        .unwrap();
        let rows: Vec<(i64,)> = conn.exec_rows("SELECT amount FROM t1 WHERE id = 2");
        assert_eq!(
            rows,
            vec![(99,)],
            "WHERE clause should compare against decoded value (10 < 20 = true)"
        );

        // WHERE clause should block update when condition is false (99 < 20 is false)
        conn.execute(
            "INSERT INTO t1 VALUES (2, 5) ON CONFLICT(id) DO UPDATE SET amount = excluded.amount WHERE t1.amount < 20",
        )
        .unwrap();
        let rows: Vec<(i64,)> = conn.exec_rows("SELECT amount FROM t1 WHERE id = 2");
        assert_eq!(
            rows,
            vec![(99,)],
            "WHERE clause should block update when decoded value 99 >= 20"
        );

        // Complex expression: excluded.amount + t1.amount
        conn.execute("DELETE FROM t1").unwrap();
        conn.execute("INSERT INTO t1 VALUES (1, 42)").unwrap();
        conn.execute(
            "INSERT INTO t1 VALUES (1, 8) ON CONFLICT(id) DO UPDATE SET amount = excluded.amount + t1.amount",
        )
        .unwrap();
        let rows: Vec<(i64,)> = conn.exec_rows("SELECT amount FROM t1 WHERE id = 1");
        assert_eq!(
            rows,
            vec![(50,)],
            "excluded.amount (8) + t1.amount (42) should equal 50"
        );
```
---
## `test:tests/integration/custom_types.rs:test_vacuum_into_with_custom_types`

```rust
let path = TempDir::new()
            .unwrap()
            .keep()
            .join("custom_types_vacuum_src.db");
        let dest_path = path.with_file_name("custom_types_vacuum_dest.db");
        let opts = turso_core::DatabaseOpts::new()
            .with_custom_types(true)
            .with_encryption(true);

        // Create source database with custom type and data
        {
            let db = TempDatabase::new_with_existent_with_opts(&path, opts);
            let conn = db.connect_limbo();
            conn.execute("CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100")
                .unwrap();
            conn.execute("CREATE TABLE t1(id INTEGER PRIMARY KEY, amount cents) STRICT")
                .unwrap();
            conn.execute("INSERT INTO t1 VALUES (1, 42)").unwrap();
            conn.execute("INSERT INTO t1 VALUES (2, 100)").unwrap();

            // VACUUM INTO destination
            conn.execute(format!("VACUUM INTO '{
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_db_share_same_file`

```rust
let mut path = TempDir::new().unwrap().keep();
    let (mut rng, _) = rng_from_time();
    path.push(format!("test-{
```
---
## `test:tests/integration/query_processing/test_alter_table_reopen.rs:test_alter_table_add_column_preserves_multiple_unique_constraints_reopen`

```rust
let path = TempDir::new()
        .unwrap()
        .keep()
        .join("alter_add_col_multi_unique_reopen.db");

    // Session 1: create table with two table-level UNIQUEs, add column, close
    {
        let db = TempDatabase::new_with_existent(&path);
        let conn = db.connect_limbo();
        conn.execute(
            "CREATE TABLE t (
                a TEXT,
                b INTEGER,
                c TEXT,
                UNIQUE (a, b),
                UNIQUE (b, c)
            )",
        )
        .unwrap();
        conn.execute("ALTER TABLE t ADD COLUMN d TEXT").unwrap();
        conn.close().unwrap();
```
---
## `test:tests/integration/query_processing/test_alter_table_reopen.rs:test_alter_table_add_column_preserves_unique_constraint_reopen`

```rust
let path = TempDir::new()
        .unwrap()
        .keep()
        .join("alter_add_col_unique_reopen.db");

    // Session 1: create table with table-level UNIQUE, add column, close
    {
        let db = TempDatabase::new_with_existent(&path);
        let conn = db.connect_limbo();
        conn.execute(
            "CREATE TABLE events (
                id TEXT,
                stream_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                PRIMARY KEY (id),
                UNIQUE (stream_id, version)
            )",
        )
        .unwrap();
        conn.execute("ALTER TABLE events ADD COLUMN extra TEXT")
            .unwrap();
        conn.close().unwrap();
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_commit_without_mvcc`

```rust
let tmp_db = TempDatabase::new("test_commit_without_mvcc.db");
    let conn = tmp_db.connect_limbo();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
        .unwrap();

    conn.execute("BEGIN IMMEDIATE").unwrap();

    assert!(
        !conn.get_auto_commit(),
        "should not be in autocommit mode after BEGIN"
    );

    conn.execute("INSERT INTO test (id, value) VALUES (1, 'hello')")
        .unwrap();

    conn.execute("COMMIT")
        .expect("COMMIT should succeed for non-MVCC transactions");

    assert!(
        conn.get_auto_commit(),
        "should be back in autocommit mode after COMMIT"
    );

    let stmt = conn
        .query("SELECT value FROM test WHERE id = 1")
        .unwrap()
        .unwrap();
    let row = helper_read_single_row(stmt);
    assert_eq!(row[0], Value::Text("hello".into()));
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_checkpoint_before_delete_then_reopen`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_checkpoint_before_delete_then_reopen.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_checkpoint_before_delete_then_verify_same_session`

```rust
let tmp_db = TempDatabase::new_with_mvcc(
        "test_mvcc_checkpoint_before_delete_then_verify_same_session.db",
    );
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_checkpoint_before_insert_delete_after_checkpoint`

```rust
let tmp_db = TempDatabase::new_with_mvcc(
        "test_mvcc_checkpoint_before_insert_delete_after_checkpoint.db",
    );
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,2)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(3,4)",
    )
    .unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 3").unwrap();

    verify_table_contents(&conn, vec![1, 4]);

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 4]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_checkpoint_delete_checkpoint_then_reopen`

```rust
let tmp_db =
        TempDatabase::new_with_mvcc("test_mvcc_checkpoint_delete_checkpoint_then_reopen.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_checkpoint_delete_checkpoint_then_verify_same_session`

```rust
let tmp_db = TempDatabase::new_with_mvcc(
        "test_mvcc_checkpoint_delete_checkpoint_then_verify_same_session.db",
    );
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_checkpoint_works`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_checkpoint_works.db");

    // Create table
    let conn = tmp_db.connect_limbo();
    conn.execute("CREATE TABLE test (id INTEGER, value TEXT)")
        .unwrap();

    // Insert rows from multiple connections
    let mut expected_rows = Vec::new();

    // Create 5 connections, each inserting 20 rows
    for conn_id in 0..5 {
        let conn = tmp_db.connect_limbo();
        conn.execute("BEGIN CONCURRENT").unwrap();

        // Each connection inserts rows with its own pattern
        for i in 0..20 {
            let id = conn_id * 100 + i;
            let value = format!("value_conn{conn_id
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_concurrent_conflicting_update`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_concurrent_conflicting_update.db");
    let conn1 = tmp_db.connect_limbo();
    let conn2 = tmp_db.connect_limbo();

    conn1
        .execute("CREATE TABLE test (id INTEGER, value TEXT)")
        .unwrap();

    conn1
        .execute("INSERT INTO test (id, value) VALUES (1, 'first')")
        .unwrap();

    conn1.execute("BEGIN CONCURRENT").unwrap();
    conn2.execute("BEGIN CONCURRENT").unwrap();

    conn1
        .execute("UPDATE test SET value = 'second' WHERE id = 1")
        .unwrap();
    let err = conn2
        .execute("UPDATE test SET value = 'third' WHERE id = 1")
        .expect_err("expected error");
    assert!(matches!(err, LimboError::WriteWriteConflict));
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_concurrent_conflicting_update_2`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_concurrent_conflicting_update.db");
    let conn1 = tmp_db.connect_limbo();
    let conn2 = tmp_db.connect_limbo();

    conn1
        .execute("CREATE TABLE test (id INTEGER, value TEXT)")
        .unwrap();

    conn1
        .execute("INSERT INTO test (id, value) VALUES (1, 'first'), (2, 'first')")
        .unwrap();

    conn1.execute("BEGIN CONCURRENT").unwrap();
    conn2.execute("BEGIN CONCURRENT").unwrap();

    conn1
        .execute("UPDATE test SET value = 'second' WHERE id = 1")
        .unwrap();
    let err = conn2
        .execute("UPDATE test SET value = 'third' WHERE id BETWEEN 0 AND 10")
        .expect_err("expected error");
    assert!(matches!(err, LimboError::WriteWriteConflict));
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_concurrent_insert_basic`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_update_basic.db");
    let conn1 = tmp_db.connect_limbo();
    let conn2 = tmp_db.connect_limbo();

    conn1
        .execute("CREATE TABLE test (id INTEGER, value TEXT)")
        .unwrap();

    conn1.execute("BEGIN CONCURRENT").unwrap();
    conn2.execute("BEGIN CONCURRENT").unwrap();

    conn1
        .execute("INSERT INTO test (id, value) VALUES (1, 'first')")
        .unwrap();
    conn2
        .execute("INSERT INTO test (id, value) VALUES (2, 'second')")
        .unwrap();

    conn1.execute("COMMIT").unwrap();
    conn2.execute("COMMIT").unwrap();

    let stmt = conn1.query("SELECT * FROM test").unwrap().unwrap();
    let rows = helper_read_all_rows(stmt);
    assert_eq!(
        rows,
        vec![
            vec![Value::from_i64(1), Value::build_text("first")],
            vec![Value::from_i64(2), Value::build_text("second")],
        ]
    );
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_delete_then_checkpoint_then_reopen`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_delete_then_checkpoint_then_reopen.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_delete_then_checkpoint_then_verify_same_session`

```rust
let tmp_db =
        TempDatabase::new_with_mvcc("test_mvcc_delete_then_checkpoint_then_verify_same_session.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_delete_then_reopen_no_checkpoint`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_delete_then_reopen_no_checkpoint.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    tracing::info!("Reopening database");
    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_delete_then_reopen_no_checkpoint_2`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_delete_then_reopen_no_checkpoint.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x unique, y unique)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value, value * 10 FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    tracing::info!("Reopening database");
    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_dual_seek_index_basic`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_dual_seek_index_basic.db");
    let conn = tmp_db.connect_limbo();

    // Create table with index
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER, v TEXT)").unwrap();
    execute_and_log(&conn, "CREATE INDEX idx_x ON t(x)").unwrap();

    // Insert initial rows
    for i in 1..=5 {
        execute_and_log(&conn, &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_dual_seek_interleaved_rows`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_dual_seek_interleaved_rows.db");
    let conn = tmp_db.connect_limbo();

    // Create table and insert odd rows
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER PRIMARY KEY, v TEXT)").unwrap();
    for i in [1, 3, 5, 7, 9] {
        execute_and_log(&conn, &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_dual_seek_range_operations`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_dual_seek_range_operations.db");
    let conn = tmp_db.connect_limbo();

    // Create table and insert rows
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER PRIMARY KEY)").unwrap();
    for i in [1, 3, 5] {
        execute_and_log(&conn, &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_dual_seek_table_rowid_basic`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_dual_seek_table_rowid_basic.db");
    let conn = tmp_db.connect_limbo();

    // Create table and insert initial rows
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER PRIMARY KEY, v TEXT)").unwrap();
    for i in 1..=5 {
        execute_and_log(&conn, &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_dual_seek_with_delete`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_dual_seek_with_delete.db");
    let conn = tmp_db.connect_limbo();

    // Create table and insert rows
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER PRIMARY KEY, v TEXT)").unwrap();
    for i in 1..=5 {
        execute_and_log(&conn, &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_dual_seek_with_update`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_dual_seek_with_update.db");
    let conn = tmp_db.connect_limbo();

    // Create table and insert rows
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER PRIMARY KEY, v TEXT)").unwrap();
    for i in 1..=5 {
        execute_and_log(
            &conn,
            &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_index_after_checkpoint_delete_after_index`

```rust
let tmp_db =
        TempDatabase::new_with_mvcc("test_mvcc_index_after_checkpoint_delete_after_index.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_index_before_checkpoint_delete_after_checkpoint`

```rust
let tmp_db =
        TempDatabase::new_with_mvcc("test_mvcc_index_before_checkpoint_delete_after_checkpoint.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_multiple_deletes_with_checkpoints`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_multiple_deletes_with_checkpoints.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,5)",
    )
    .unwrap();
    execute_and_log(&conn, "CREATE INDEX lol ON t(x)").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 4").unwrap();

    verify_table_contents(&conn, vec![1, 3, 5]);

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3, 5]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_no_index_checkpoint_delete_reopen`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_no_index_checkpoint_delete_reopen.db");
    let conn = tmp_db.connect_limbo();

    execute_and_log(&conn, "CREATE TABLE t (x)").unwrap();
    execute_and_log(
        &conn,
        "INSERT INTO t SELECT value FROM generate_series(1,3)",
    )
    .unwrap();
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();
    execute_and_log(&conn, "DELETE FROM t WHERE x = 2").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    verify_table_contents(&conn, vec![1, 3]);
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_recovery_of_both_checkpointed_and_noncheckpointed_tables_works`

```rust
let tmp_db = TempDatabase::new_with_mvcc(
        "test_mvcc_recovery_of_both_checkpointed_and_noncheckpointed_tables_works.db",
    );
    let conn = tmp_db.connect_limbo();

    // Create first table and insert rows
    execute_and_log(
        &conn,
        "CREATE TABLE test1 (id INTEGER PRIMARY KEY, value INTEGER)",
    )
    .unwrap();

    let mut expected_rows1 = Vec::new();
    for i in 0..10 {
        let value = i * 10;
        execute_and_log(
            &conn,
            &format!("INSERT INTO test1 (id, value) VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_mvcc_recovery_with_index_and_deletes`

```rust
let tmp_db = TempDatabase::new_with_mvcc("test_mvcc_recovery_with_index_and_deletes.db");
    let conn = tmp_db.connect_limbo();

    // Create table with unique constraint (creates an index)
    execute_and_log(&conn, "CREATE TABLE t (x INTEGER UNIQUE)").unwrap();

    // Insert 5 values
    for i in 1..=5 {
        execute_and_log(&conn, &format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_non_mvcc_to_mvcc`

```rust
// Create non-mvcc database
    let tmp_db = TempDatabase::new("test_non_mvcc_to_mvcc.db");
    let conn = tmp_db.connect_limbo();

    // Create table and insert data
    execute_and_log(
        &conn,
        "CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)",
    )
    .unwrap();
    execute_and_log(&conn, "INSERT INTO test VALUES (1, 'hello')").unwrap();

    // Checkpoint to persist changes
    execute_and_log(&conn, "PRAGMA wal_checkpoint(TRUNCATE)").unwrap();

    let path = tmp_db.path.clone();
    drop(conn);
    drop(tmp_db);

    // Reopen in mvcc mode
    let tmp_db = TempDatabase::new_with_existent(&path);
    let conn = tmp_db.connect_limbo();

    // Query should work
    let stmt = query_and_log(&conn, "SELECT * FROM test").unwrap().unwrap();
    let rows = helper_read_all_rows(stmt);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::from_i64(1));
    assert_eq!(rows[0][1], Value::Text("hello".into()));
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_rollback_without_mvcc`

```rust
let tmp_db = TempDatabase::new("test_rollback_without_mvcc.db");
    let conn = tmp_db.connect_limbo();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)")
        .unwrap();

    conn.execute("INSERT INTO test (id, value) VALUES (1, 'initial')")
        .unwrap();

    conn.execute("BEGIN IMMEDIATE").unwrap();

    assert!(
        !conn.get_auto_commit(),
        "should not be in autocommit mode after BEGIN"
    );

    conn.execute("UPDATE test SET value = 'modified' WHERE id = 1")
        .unwrap();

    conn.execute("ROLLBACK")
        .expect("ROLLBACK should succeed for non-MVCC transactions");

    assert!(
        conn.get_auto_commit(),
        "should be back in autocommit mode after ROLLBACK"
    );

    let stmt = conn
        .query("SELECT value FROM test WHERE id = 1")
        .unwrap()
        .unwrap();
    let row = helper_read_single_row(stmt);
    assert_eq!(row[0], Value::Text("initial".into()));
```
---
## `test:tests/integration/query_processing/test_transactions.rs:test_wal_savepoint_rollback_on_constraint_violation`

```rust
let tmp_db = TempDatabase::new("test_90969.db");
    let conn = tmp_db.connect_limbo();

    conn.execute("PRAGMA cache_size = 200").unwrap();

    conn.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, u INTEGER UNIQUE, val TEXT)")
        .unwrap();

    let padding = "x".repeat(2000);
    conn.execute("BEGIN").unwrap();
    for i in 1..=1000 {
        conn.execute(format!("INSERT INTO t VALUES ({i
```
---
## `test:tests/integration/query_processing/test_write_path.rs:test_unique_complex_key`

```rust
let _ = env_logger::try_init();
    let db_path = tempfile::NamedTempFile::new().unwrap();
    {
        let connection = rusqlite::Connection::open(db_path.path()).unwrap();
        connection
            .execute("CREATE TABLE t(a, b, c, UNIQUE (b, a));", ())
            .unwrap();
        connection
            .execute("INSERT INTO t VALUES ('1', '2', 'a'), ('3', '4', 'b');", ())
            .unwrap();
```
---
## `test:tests/integration/storage/autovacuum.rs:test_autovacuum_readonly_behavior`

```rust
// (autovacuum_mode, enable_autovacuum_flag, expected_readonly)
    // TODO: Add encrypted case ("NONE", false, false) after fixing https://github.com/tursodatabase/turso/issues/4519
    let test_cases = [
        ("NONE", false, false),
        ("NONE", true, false),
        ("FULL", false, true),
        ("FULL", true, false),
        ("INCREMENTAL", false, true),
        ("INCREMENTAL", true, false),
    ];

    for (autovacuum_mode, enable_autovacuum_flag, expected_readonly) in test_cases {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.pragma_update(None, "auto_vacuum", autovacuum_mode)
                .unwrap();
```
---
## `test:tests/integration/storage/checksum.rs:test_checksum_detects_corruption`

```rust
let _ = env_logger::try_init();
    let db_name = format!("test-corruption-{
```
---
## `test:tests/integration/storage/header_version.rs:test_legacy_db_opened_with_mvcc_converts_to_mvcc`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a Legacy mode database
    create_legacy_db(&db_path);

    // Open with limbo (with MVCC enabled)
    let (write_ver, read_ver) = open_with_limbo_and_check(&db_path, true);

    // Should be converted to MVCC mode (version 255)
    assert_eq!(
        write_ver, 255,
        "Legacy DB opened with MVCC should convert to MVCC (write_version=255), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_legacy_db_opened_without_mvcc_converts_to_wal`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a Legacy mode database
    create_legacy_db(&db_path);

    // Open with limbo (without MVCC)
    let (write_ver, read_ver) = open_with_limbo_and_check(&db_path, false);

    // Should be converted to WAL mode (version 2)
    assert_eq!(
        write_ver, 2,
        "Legacy DB opened without MVCC should convert to WAL (write_version=2), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_mvcc_db_opened_with_mvcc_stays_mvcc`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database first
    create_wal_db(&db_path);

    // Open with limbo with MVCC to convert it to MVCC mode
    let _ = open_with_limbo_and_check(&db_path, true);

    // Verify it's now MVCC
    let (write_ver, read_ver) = read_header_versions(&db_path);
    assert_eq!(write_ver, 255, "Should be MVCC after first open");
    assert_eq!(read_ver, 255, "Should be MVCC after first open");

    // Open again with MVCC flag - should stay MVCC
    let (write_ver, read_ver) = open_with_limbo_and_check(&db_path, true);

    assert_eq!(
        write_ver, 255,
        "MVCC DB opened with MVCC should stay MVCC (write_version=255), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_mvcc_db_opened_without_mvcc_flag_stays_mvcc`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database first
    create_wal_db(&db_path);

    // Open with limbo with MVCC to convert it to MVCC mode
    let _ = open_with_limbo_and_check(&db_path, true);

    // Verify it's now MVCC
    let (write_ver, read_ver) = read_header_versions(&db_path);
    assert_eq!(write_ver, 255, "Should be MVCC after first open");
    assert_eq!(read_ver, 255, "Should be MVCC after first open");

    // Now open WITHOUT MVCC flag - should auto-enable MVCC and stay at version 255
    let (write_ver, read_ver) = open_with_limbo_and_check(&db_path, false);

    assert_eq!(
        write_ver, 255,
        "MVCC DB opened without MVCC flag should stay MVCC (write_version=255), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_pragma_journal_mode_data_persistence_after_switch`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    // Open and switch to MVCC, insert data
    {
        let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
        let opts = DatabaseOpts::new();

        let db = Database::open_file_with_flags(
            io.clone(),
            db_path.to_str().unwrap(),
            OpenFlags::default(),
            opts,
            None,
        )
        .expect("Failed to open database");

        let conn = db.connect().unwrap();

        // Switch to MVCC
        conn.pragma_update("journal_mode", "'mvcc'")
            .expect("Switch to MVCC should work");

        // Insert data after switch
        conn.execute("INSERT INTO t (val) VALUES ('persisted_data')")
            .expect("INSERT should work");

        drop(conn);
        drop(db);
```
---
## `test:tests/integration/storage/header_version.rs:test_pragma_journal_mode_multiple_switches`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
    let opts = DatabaseOpts::new();

    let db = Database::open_file_with_flags(
        io.clone(),
        db_path.to_str().unwrap(),
        OpenFlags::default(),
        opts,
        None,
    )
    .expect("Failed to open database");

    let conn = db.connect().unwrap();

    // Switch to MVCC
    let result = conn
        .pragma_update("journal_mode", "'mvcc'")
        .expect("Switch to MVCC should work");
    assert_eq!(result[0][0].to_string(), "mvcc");

    // Verify header is MVCC (version 255)
    let (write_ver, read_ver) = read_header_versions(&db_path);
    assert_eq!(write_ver, 255, "mode should be MVCC (write_version=255)");
    assert_eq!(read_ver, 255, "mode should be MVCC (read_version=255)");

    // Insert data in MVCC mode
    conn.execute("INSERT INTO t (val) VALUES ('after_mvcc_switch')")
        .expect("INSERT should work");

    // Switch back to WAL
    let result = conn
        .pragma_update("journal_mode", "'wal'")
        .expect("Switch to WAL should work");
    assert_eq!(result[0][0].to_string(), "wal");

    // Verify header is MVCC (version 255)
    let (write_ver, read_ver) = read_header_versions(&db_path);
    assert_eq!(write_ver, 2, "mode should be WAL (write_version=2)");
    assert_eq!(read_ver, 2, "mode should be WAL (read_version=2)");

    // Insert data in WAL mode
    conn.execute("INSERT INTO t (val) VALUES ('after_wal_switch')")
        .expect("INSERT should work");

    // Switch to MVCC again
    let result = conn
        .pragma_update("journal_mode", "'mvcc'")
        .expect("Switch to MVCC again should work");
    assert_eq!(result[0][0].to_string(), "mvcc");

    let (write_ver, read_ver) = read_header_versions(&db_path);
    assert_eq!(write_ver, 255, "mode should be MVCC (write_version=255)");
    assert_eq!(read_ver, 255, "mode should be MVCC (read_version=255)");

    // Insert data in MVCC mode
    conn.execute("INSERT INTO t (val) VALUES ('after_second_mvcc_switch')")
        .expect("INSERT should work");

    // Verify all data is present
    let mut stmt = conn.prepare("SELECT val FROM t ORDER BY val").unwrap();
    let mut rows = Vec::new();
    stmt.run_with_row_callback(|row| {
        let val: String = row.get::<String>(0).unwrap();
        rows.push(val);
        Ok(())
```
---
## `test:tests/integration/storage/header_version.rs:test_pragma_journal_mode_mvcc_to_wal`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Step 1: Create a WAL mode database and convert to MVCC
    create_wal_db(&db_path);

    // Step 2: Open and switch to MVCC mode via PRAGMA, then add some data
    {
        let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
        let opts = DatabaseOpts::new();

        let db = Database::open_file_with_flags(
            io.clone(),
            db_path.to_str().unwrap(),
            OpenFlags::default(),
            opts,
            None,
        )
        .expect("Failed to open database with limbo");

        let conn = db.connect().unwrap();

        // Switch to MVCC mode via PRAGMA
        conn.pragma_update("journal_mode", "'mvcc'")
            .expect("PRAGMA journal_mode = 'mvcc' should work");

        // Insert some data in MVCC mode
        conn.execute("INSERT INTO t (val) VALUES ('mvcc_data')")
            .expect("INSERT should work in MVCC mode");

        drop(conn);
        drop(db);
```
---
## `test:tests/integration/storage/header_version.rs:test_pragma_journal_mode_query`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
    let opts = DatabaseOpts::new();

    let db = Database::open_file_with_flags(
        io.clone(),
        db_path.to_str().unwrap(),
        OpenFlags::default(),
        opts,
        None,
    )
    .expect("Failed to open database");

    let conn = db.connect().unwrap();

    // Query current journal mode (should be WAL)
    if let Some(mut stmt) = conn.query("PRAGMA journal_mode").unwrap() {
        stmt.run_with_row_callback(|row| {
            let mode: String = row.get::<String>(0).unwrap();
            assert_eq!(mode, "wal", "Initial mode should be WAL, got {mode
```
---
## `test:tests/integration/storage/header_version.rs:test_pragma_journal_mode_wal_to_mvcc_with_pending_wal`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Step 1: Create a WAL mode database with rusqlite
    create_wal_db_with_pending_wal(&db_path);

    // Step 2: Open with limbo WITHOUT MVCC to create WAL data, then close without checkpointing
    {
        let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
        let opts = DatabaseOpts::new();

        let db = Database::open_file_with_flags(
            io.clone(),
            db_path.to_str().unwrap(),
            OpenFlags::default(),
            opts,
            None,
        )
        .expect("Failed to open database with limbo (non-MVCC)");

        let conn = db.connect().unwrap();

        // Insert some data to ensure WAL has content
        conn.execute("INSERT INTO t (val) VALUES ('limbo_data')")
            .expect("INSERT should work");

        // Drop without checkpointing - this should leave data in WAL
        drop(conn);
        drop(db);
```
---
## `test:tests/integration/storage/header_version.rs:test_readonly_legacy_db_header_not_modified`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a Legacy mode database
    create_legacy_db(&db_path);

    // Get the original header versions
    let (orig_write_ver, orig_read_ver) = read_header_versions(&db_path);
    assert_eq!(orig_write_ver, 1, "Original should be Legacy mode");
    assert_eq!(orig_read_ver, 1, "Original should be Legacy mode");

    // Open with limbo in readonly mode (without MVCC)
    let (write_ver, read_ver) = open_with_limbo_readonly_and_check(&db_path, false);

    // Header should NOT be modified - should still be Legacy mode
    assert_eq!(
        write_ver, 1,
        "Readonly DB should NOT convert Legacy to WAL (write_version should stay 1), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_readonly_legacy_db_with_mvcc_header_not_modified`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a Legacy mode database
    create_legacy_db(&db_path);

    // Get the original header versions
    let (orig_write_ver, orig_read_ver) = read_header_versions(&db_path);
    assert_eq!(orig_write_ver, 1, "Original should be Legacy mode");
    assert_eq!(orig_read_ver, 1, "Original should be Legacy mode");

    // Open with limbo in readonly mode with MVCC enabled
    let (write_ver, read_ver) = open_with_limbo_readonly_and_check(&db_path, true);

    // Header should NOT be modified - should still be Legacy mode
    // even though MVCC was requested
    assert_eq!(
        write_ver, 1,
        "Readonly DB should NOT convert Legacy to MVCC (write_version should stay 1), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_readonly_mvcc_db_can_be_read`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database and convert to MVCC
    create_wal_db(&db_path);

    // First, convert to MVCC by opening in read-write mode and using PRAGMA
    {
        let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
        let opts = DatabaseOpts::new();

        let db = Database::open_file_with_flags(
            io.clone(),
            db_path.to_str().unwrap(),
            OpenFlags::default(),
            opts,
            None,
        )
        .expect("Failed to open database");

        let conn = db.connect().unwrap();

        // Switch to MVCC mode via PRAGMA
        conn.pragma_update("journal_mode", "'mvcc'")
            .expect("PRAGMA journal_mode = 'mvcc' should work");

        // Insert some data in MVCC mode
        conn.execute("INSERT INTO t (val) VALUES ('mvcc_readonly_test')")
            .expect("INSERT should work in MVCC mode");

        drop(conn);
        drop(db);
```
---
## `test:tests/integration/storage/header_version.rs:test_readonly_pragma_journal_mode_cannot_change`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
    let opts = DatabaseOpts::new();

    let db = Database::open_file_with_flags(
        io.clone(),
        db_path.to_str().unwrap(),
        OpenFlags::ReadOnly,
        opts,
        None,
    )
    .expect("Failed to open database in readonly mode");

    let conn = db.connect().unwrap();

    // Try to switch to MVCC mode via PRAGMA - this should return an error
    // because we cannot change mode on readonly databases
    let result = conn.pragma_update("journal_mode", "'mvcc'");

    // The result should be a ReadOnly error
    assert!(
        matches!(result, Err(turso_core::LimboError::ReadOnly)),
        "PRAGMA journal_mode should return ReadOnly error on readonly database, got: {result:?
```
---
## `test:tests/integration/storage/header_version.rs:test_readonly_wal_db_with_mvcc_header_not_modified`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    // Get the original header versions
    let (orig_write_ver, orig_read_ver) = read_header_versions(&db_path);
    assert_eq!(orig_write_ver, 2, "Original should be WAL mode");
    assert_eq!(orig_read_ver, 2, "Original should be WAL mode");

    // Open with limbo in readonly mode with MVCC enabled
    let (write_ver, read_ver) = open_with_limbo_readonly_and_check(&db_path, true);

    // Header should NOT be modified - should still be WAL mode
    // even though MVCC was requested
    assert_eq!(
        write_ver, 2,
        "Readonly DB should NOT convert WAL to MVCC (write_version should stay 2), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_utf16_db_returns_unsupported_encoding_error`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    create_utf16le_db(&db_path);

    let io = std::sync::Arc::new(turso_core::PlatformIO::new().unwrap());
    let opts = DatabaseOpts::new();

    let result = Database::open_file_with_flags(
        io.clone(),
        db_path.to_str().unwrap(),
        OpenFlags::default(),
        opts,
        None,
    );

    assert!(
        matches!(result, Err(turso_core::LimboError::UnsupportedEncoding(_))),
        "Opening UTF-16 database should return UnsupportedEncoding error, got: {result:?
```
---
## `test:tests/integration/storage/header_version.rs:test_wal_db_opened_with_mvcc_converts_to_mvcc`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    // Open with limbo (with MVCC enabled)
    let (write_ver, read_ver) = open_with_limbo_and_check(&db_path, true);

    // Should be converted to MVCC mode (version 255)
    assert_eq!(
        write_ver, 255,
        "WAL DB opened with MVCC should convert to MVCC (write_version=255), got {write_ver
```
---
## `test:tests/integration/storage/header_version.rs:test_wal_db_opened_without_mvcc_stays_wal`

```rust
let tmp_dir = TempDir::new().unwrap();
    let db_path = tmp_dir.path().join("test.db");

    // Create a WAL mode database
    create_wal_db(&db_path);

    // Open with limbo (without MVCC)
    let (write_ver, read_ver) = open_with_limbo_and_check(&db_path, false);

    // Should stay WAL mode (version 2)
    assert_eq!(
        write_ver, 2,
        "WAL DB opened without MVCC should stay WAL (write_version=2), got {write_ver
```
---
## `test:tests/integration/storage/short_read.rs:test_truncated_database_returns_short_read_error`

```rust
let _ = env_logger::try_init();
    let db_name = format!("test-truncated-{
```
---
## `test:tests/integration/storage/short_read.rs:test_truncated_header_returns_short_read_error`

```rust
let _ = env_logger::try_init();
    let db_name = format!("test-truncated-header-{
```
---
## `test:tests/integration/storage/short_read.rs:test_truncated_wal_returns_short_read_error`

```rust
let _ = env_logger::try_init();
    let db_name = format!("test-truncated-wal-{
```
---
## `test:tests/integration/storage/short_read.rs:test_zeroed_page_returns_corrupt_error`

```rust
let _ = env_logger::try_init();
    let db_name = format!("test-zeroed-page-{
```
---
## `test:tests/integration/wal/test_wal.rs:test_wal_read_lock_released_on_conn_drop`

```rust
maybe_setup_tracing();
    let tmp_db = TempDatabase::new("test_wal_read_lock_released.db");
    let db = tmp_db.limbo_database();

    // Setup: create table and insert data so WAL has content
    let setup_conn = db.connect().unwrap();
    setup_conn
        .execute("CREATE TABLE t (id integer primary key)")
        .unwrap();
    setup_conn.execute("INSERT INTO t VALUES (1)").unwrap();

    let conn1 = db.connect().unwrap();
    let conn2 = db.connect().unwrap();

    // conn1 starts a read transaction and panics while holding the read lock
    let join_result = std::thread::spawn(move || {
        conn1.execute("BEGIN").unwrap();
        conn1.execute("SELECT * FROM t").unwrap();
        panic!("intentional panic while holding read tx");
```
---
## `test:tests/integration/wal/test_wal.rs:test_wal_write_lock_released_on_conn_drop`

```rust
maybe_setup_tracing();
    let tmp_db = TempDatabase::new("test_wal_write_lock_released.db");
    let db = tmp_db.limbo_database();

    let conn1 = db.connect().unwrap();
    let conn2 = db.connect().unwrap();

    let join_result = std::thread::spawn(move || {
        conn1.execute("BEGIN IMMEDIATE").unwrap();
        panic!("intentional panic while holding write tx");
```
---
## `text_file:AGENTS.md:8:0`

```markdown
```bash
cargo build                    # build. never build with release.
cargo test                     # rust unit/integration tests
cargo fmt                      # format (required)
cargo clippy --workspace --all-features --all-targets -- --deny=warnings  # lint
cargo run -q --bin tursodb -- -q # run the interactive cli

make test                      # TCL compat + sqlite3 + extensions + MVCC
make test-single TEST=foo.test # single TCL test
make -C testing/runner run-rust  # sqltest runner (preferred for new tests)```
```
---
## `text_file:CONTRIBUTING.md:347:21`

```markdown
```bash
python3.12 -m venv venv
source venv/bin/activate```
```
---
## `text_file:CONTRIBUTING.md:354:22`

```markdown
```bash
pip install maturin```
```
---
## `text_file:CONTRIBUTING.md:360:23`

```markdown
```bash
cd bindings/python && maturin develop```
```
---
## `text_file:CONTRIBUTING.md:391:24`

```markdown
```bash
export ANTITHESIS_USER=
export ANTITHESIS_TENANT=
export ANTITHESIS_PASSWD=
export ANTITHESIS_DOCKER_HOST=
export ANTITHESIS_DOCKER_REPO=
export ANTITHESIS_EMAIL=```
```
---
## `text_file:CONTRIBUTING.md:402:25`

```markdown
```bash
scripts/antithesis/publish-workload.sh```
```
---
## `text_file:CONTRIBUTING.md:408:26`

```markdown
```bash
scripts/antithesis/launch.sh```
```
---
## `text_file:CONTRIBUTING.md:51:0`

```markdown
```shell
cargo run --package turso_cli --bin tursodb database.db```
```
---
## `text_file:CONTRIBUTING.md:82:3`

```markdown
```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]```
```
---
## `text_file:CONTRIBUTING.md:90:4`

```markdown
```toml
[target.x86_64-apple-darwin]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[target.aarch64-apple-darwin]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]```
```
---
## `text_file:PERF.md:38:3`

```markdown
```shell
make clickbench```
```
---
## `text_file:PERF.md:48:4`

```markdown
```shell
make bench-vfs SQL="select * from users;" N=500```
```
---
## `text_file:PERF.md:62:5`

```markdown
```shell
./perf/tpc-h/benchmark.sh```
```
---
## `text_file:bindings/javascript/docs/API.md:156:1`

```markdown
```js
stmt.pluck(); // plucking ON
stmt.pluck(true); // plucking ON
stmt.pluck(false); // plucking OFF```
```
---
## `text_file:bindings/javascript/docs/API.md:178:2`

```markdown
```js
stmt.raw(); // raw mode ON
stmt.raw(true); // raw mode ON
stmt.raw(false); // raw mode OFF```
```
---
## `text_file:bindings/javascript/docs/API.md:51:0`

```markdown
```js
db.pragma('cache_size = 32000');
console.log(db.pragma('cache_size', { simple: true })); // => 32000```
```
---
## `text_file:bindings/javascript/docs/CONTRIBUTING.md:8:0`

```markdown
```sh
yarn global add @napi-rs/cli```
```
---
## `text_file:bindings/python/SQLALCHEMY_DIALECT.md:136:4`

```markdown
```python
from turso.sqlalchemy import get_sync_connection

with engine.connect() as conn:
    sync = get_sync_connection(conn)

    # Pull changes from remote (returns True if updates were pulled)
    if sync.pull():
        print("Pulled new changes!")

    # Push local changes to remote
    sync.push()

    # Checkpoint the WAL
    sync.checkpoint()

    # Get sync statistics
    stats = sync.stats()
    print(f"Network received: {stats.network_received_bytes} bytes")```
```
---
## `text_file:bindings/python/SQLALCHEMY_DIALECT.md:14:0`

```markdown
```bash
pip install pyturso[sqlalchemy]```
```
---
## `text_file:bindings/python/SQLALCHEMY_DIALECT.md:22:1`

```markdown
```python
from sqlalchemy import create_engine, text

# In-memory database
engine = create_engine("sqlite+turso:///:memory:")

# File-based database
engine = create_engine("sqlite+turso:///path/to/database.db")

with engine.connect() as conn:
    conn.execute(text("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)"))
    conn.execute(text("INSERT INTO users (name) VALUES ('Alice')"))
    conn.commit()

    result = conn.execute(text("SELECT * FROM users"))
    for row in result:
        print(row)```
```
---
## `text_file:bindings/python/SQLALCHEMY_DIALECT.md:257:5`

```markdown
```toml
[project.entry-points."sqlalchemy.dialects"]
"sqlite.turso" = "turso.sqlalchemy:TursoDialect"
"sqlite.turso_sync" = "turso.sqlalchemy:TursoSyncDialect"```
```
---
## `text_file:bindings/python/SQLALCHEMY_DIALECT.md:43:2`

```markdown
```python
from sqlalchemy import create_engine, text
from turso.sqlalchemy import get_sync_connection

# Via URL query parameters
engine = create_engine(
    "sqlite+turso_sync:///local.db"
    "?remote_url=https://your-db.turso.io"
    "&auth_token=your-token"
)

# Or via connect_args (supports callables for dynamic tokens)
engine = create_engine(
    "sqlite+turso_sync:///local.db",
    connect_args={
        "remote_url": "https://your-db.turso.io",
        "auth_token": lambda: get_fresh_token(),
    }
)

with engine.connect() as conn:
    # Access sync operations
    sync = get_sync_connection(conn)
    sync.pull()  # Pull changes from remote

    result = conn.execute(text("SELECT * FROM users"))

    conn.execute(text("INSERT INTO users (name) VALUES ('Bob')"))
    conn.commit()
    sync.push()  # Push changes to remote```
```
---
## `text_file:bindings/python/SQLALCHEMY_DIALECT.md:77:3`

```markdown
```python
from sqlalchemy import create_engine, Column, Integer, String
from sqlalchemy.orm import declarative_base, Session

Base = declarative_base()

class User(Base):
    __tablename__ = "users"
    id = Column(Integer, primary_key=True)
    name = Column(String(100))

engine = create_engine("sqlite+turso:///:memory:")
Base.metadata.create_all(engine)

with Session(engine) as session:
    session.add(User(name="Alice"))
    session.commit()

    users = session.query(User).all()```
```
---
## `text_file:cli/docs/config.md:80:0`

```markdown
```toml
[highlight]
theme = "Amy"```
```
---
## `text_file:cli/docs/config.md:99:1`

```markdown
```toml
[table]
column_colors = ["cyan", "black", "#010101"]
header_color = "red"

[highlight]
enable = true
prompt = "bright-blue"
theme = "base16-ocean.light"
hint = "123"
candidate = "dark-yellow"```
```
---
## `text_file:cli/docs/internal/commands.md:19:0`

```markdown
```rust
pub enum Command {
    ...
    /// Descriptive Message for your command
    Example(ExampleArgs),
   }```
```
---
## `text_file:cli/docs/internal/commands.md:29:1`

```markdown
```rust
#[derive(Debug, Clone, Args)]
    pub struct ExampleArgs {
        /// Example arg
        pub example: String,
    }```
```
---
## `text_file:cli/docs/internal/commands.md:39:2`

```markdown
```rust
pub fn handle_dot_command(&mut self, line: &str) {
        ...
        Ok(cmd) => match cmd.command {
            ...
            Command::Example(args) => {
                println!("{}", args.example);
            }
        }
    }```
```
---
## `text_file:cli/manuals/cdc.md:115:8`

```markdown
```sql
-- Create a table
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    name TEXT,
    email TEXT
);

-- Enable full CDC
PRAGMA capture_data_changes_conn('full');

-- Make some changes
INSERT INTO users VALUES (1, 'Alice', 'alice@example.com');
INSERT INTO users VALUES (2, 'Bob', 'bob@example.com');
UPDATE users SET email = 'alice@newdomain.com' WHERE id = 1;
DELETE FROM users WHERE id = 2;

-- View the captured changes
SELECT change_type, table_name, id
FROM turso_cdc;

-- Results will show:
-- 1 (INSERT) for Alice
-- 1 (INSERT) for Bob
-- 0 (UPDATE) for Alice's email change
-- -1 (DELETE) for Bob```
```
---
## `text_file:cli/manuals/cdc.md:147:9`

```markdown
```sql
-- Connection 1: Capture to 'audit_log' table
PRAGMA capture_data_changes_conn('full,audit_log');

-- Connection 2: Capture to 'sync_queue' table
PRAGMA capture_data_changes_conn('id,sync_queue');

-- Changes from Connection 1 go to 'audit_log'
-- Changes from Connection 2 go to 'sync_queue'```
```
---
## `text_file:cli/manuals/cdc.md:162:10`

```markdown
```sql
BEGIN;
INSERT INTO users VALUES (3, 'Charlie', 'charlie@example.com');
UPDATE users SET name = 'Charles' WHERE id = 3;
-- CDC table is not yet updated

COMMIT;
-- Now both the INSERT and UPDATE appear in the CDC table```
```
---
## `text_file:cli/manuals/cdc.md:16:0`

```markdown
```sql
PRAGMA capture_data_changes_conn('<mode>[,<table_name>]');```
```
---
## `text_file:cli/manuals/cdc.md:178:11`

```markdown
```sql
PRAGMA capture_data_changes_conn('full');

CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT);
-- Recorded in CDC as change to sqlite_schema

DROP TABLE products;
-- Also recorded as a schema change```
```
---
## `text_file:cli/manuals/cdc.md:38:1`

```markdown
```sql
PRAGMA capture_data_changes_conn('id');```
```
---
## `text_file:cli/manuals/cdc.md:45:2`

```markdown
```sql
PRAGMA capture_data_changes_conn('before');```
```
---
## `text_file:cli/manuals/cdc.md:50:3`

```markdown
```sql
PRAGMA capture_data_changes_conn('after');```
```
---
## `text_file:cli/manuals/cdc.md:55:4`

```markdown
```sql
PRAGMA capture_data_changes_conn('full');```
```
---
## `text_file:cli/manuals/cdc.md:62:5`

```markdown
```sql
PRAGMA capture_data_changes_conn('full,my_changes_table');```
```
---
## `text_file:cli/manuals/cdc.md:69:6`

```markdown
```sql
PRAGMA capture_data_changes_conn('off');```
```
---
## `text_file:cli/manuals/cdc.md:92:7`

```markdown
```sql
-- View all captured changes
SELECT * FROM turso_cdc;

-- View only inserts
SELECT * FROM turso_cdc WHERE change_type = 1;

-- View only updates
SELECT * FROM turso_cdc WHERE change_type = 0;

-- View only deletes
SELECT * FROM turso_cdc WHERE change_type = -1;

-- View changes for a specific table
SELECT * FROM turso_cdc WHERE table_name = 'users';

-- View recent changes (last hour)
SELECT * FROM turso_cdc
WHERE change_time > unixepoch() - 3600;```
```
---
## `text_file:cli/manuals/custom-types.md:102:6`

```markdown
```sql
CREATE TYPE uint BASE text
    ENCODE test_uint_encode(value)
    DECODE test_uint_decode(value)
    OPERATOR '+' (uint) -> test_uint_add
    OPERATOR '<' (uint) -> test_uint_lt
    OPERATOR '=' (uint) -> test_uint_eq;

CREATE TABLE t1(val uint) STRICT;
INSERT INTO t1 VALUES (20);
INSERT INTO t1 VALUES (30);

SELECT val + val FROM t1;
-- 40
-- 60

SELECT val FROM t1 WHERE val < 25;
-- 20```
```
---
## `text_file:cli/manuals/custom-types.md:132:7`

```markdown
```sql
-- ENCODE value * 100 is monotonic: 10→100, 20→200, 30→300.
-- Sorting encoded integers preserves numeric order.
CREATE TYPE cents BASE integer
    ENCODE value * 100
    DECODE value / 100
    OPERATOR '<';

CREATE TABLE prices(id INTEGER PRIMARY KEY, amount cents) STRICT;
INSERT INTO prices VALUES (1, 30), (2, 10), (3, 20);
SELECT amount FROM prices ORDER BY amount;
-- 10
-- 20
-- 30```
```
---
## `text_file:cli/manuals/custom-types.md:150:8`

```markdown
```sql
-- string_reverse is NOT monotonic: encoded text sorts differently than decoded.
-- Encoded: apple→elppa, banana→ananab, cherry→yrrehc.
-- Encoded text sort: ananab < elppa < yrrehc → display: banana, apple, cherry.
CREATE TYPE reversed BASE text
    ENCODE string_reverse(value)
    DECODE string_reverse(value)
    OPERATOR '<';

CREATE TABLE t(id INTEGER PRIMARY KEY, val reversed) STRICT;
INSERT INTO t VALUES (1, 'apple'), (2, 'banana'), (3, 'cherry');
SELECT val FROM t ORDER BY val;
-- banana
-- apple
-- cherry```
```
---
## `text_file:cli/manuals/custom-types.md:171:9`

```markdown
```sql
-- numeric stores values as blobs; standard blob comparison is wrong.
-- numeric_lt knows how to compare encoded blobs numerically.
CREATE TYPE numeric(precision, scale) BASE blob
    ENCODE numeric_encode(value, precision, scale)
    DECODE numeric_decode(value)
    OPERATOR '<' numeric_lt;```
```
---
## `text_file:cli/manuals/custom-types.md:182:10`

```markdown
```sql
-- Same encoding as above, but the comparator reverses encoded values
-- before comparing, recovering alphabetical order.
CREATE TYPE reversed_alpha BASE text
    ENCODE string_reverse(value)
    DECODE string_reverse(value)
    OPERATOR '<' string_reverse;

CREATE TABLE t(id INTEGER PRIMARY KEY, val reversed_alpha) STRICT;
INSERT INTO t VALUES (1, 'apple'), (2, 'banana'), (3, 'cherry');
SELECT val FROM t ORDER BY val;
-- apple
-- banana
-- cherry```
```
---
## `text_file:cli/manuals/custom-types.md:202:11`

```markdown
```sql
CREATE TYPE mytype BASE text ENCODE value DECODE value;
CREATE TABLE t(val mytype) STRICT;

SELECT val FROM t ORDER BY val;
-- Error: cannot ORDER BY column 'val' of type 'mytype': type does not declare OPERATOR '<'

CREATE INDEX idx ON t(val);
-- Error: cannot create index on column 'val' of type 'mytype': type does not declare OPERATOR '<'```
```
---
## `text_file:cli/manuals/custom-types.md:215:12`

```markdown
```sql
CREATE INDEX idx ON t(length(val));  -- OK: length() returns an integer```
```
---
## `text_file:cli/manuals/custom-types.md:22:0`

```markdown
```sql
CREATE TYPE type_name BASE base_type
    ENCODE encode_expr
    DECODE decode_expr
    [OPERATOR 'op' [function_name] ...]
    [DEFAULT default_expr];```
```
---
## `text_file:cli/manuals/custom-types.md:231:13`

```markdown
```sql
CREATE TYPE uint BASE text
    ENCODE test_uint_encode(value)
    DECODE test_uint_decode(value)
    DEFAULT 0;

CREATE TABLE t1(id INTEGER PRIMARY KEY, val uint) STRICT;
INSERT INTO t1(id) VALUES (1);
SELECT id, val FROM t1;
-- 1|0```
```
---
## `text_file:cli/manuals/custom-types.md:247:14`

```markdown
```sql
CREATE TABLE t1(id INTEGER PRIMARY KEY, val uint DEFAULT 42) STRICT;
INSERT INTO t1(id) VALUES (1);
SELECT id, val FROM t1;
-- 1|42```
```
---
## `text_file:cli/manuals/custom-types.md:258:15`

```markdown
```sql
CREATE TYPE reversed BASE text
    ENCODE string_reverse(value)
    DECODE string_reverse(value)
    DEFAULT string_reverse('auto');

CREATE TABLE t1(id INTEGER PRIMARY KEY, val reversed) STRICT;
INSERT INTO t1(id) VALUES (1);
SELECT id, val FROM t1;
-- 1|otua```
```
---
## `text_file:cli/manuals/custom-types.md:274:16`

```markdown
```sql
CREATE TYPE positive_int BASE integer
    ENCODE CASE WHEN value > 0 THEN value
                ELSE RAISE(ABORT, 'value must be positive') END
    DECODE value;

CREATE TABLE t1(val positive_int) STRICT;
INSERT INTO t1 VALUES (42);   -- OK
INSERT INTO t1 VALUES (-1);   -- Error: value must be positive```
```
---
## `text_file:cli/manuals/custom-types.md:287:17`

```markdown
```sql
-- varchar checks length against the maxlen parameter
CREATE TYPE varchar(maxlen) BASE text
    ENCODE CASE WHEN length(value) <= maxlen THEN value
                ELSE RAISE(ABORT, 'value too long for varchar') END
    DECODE value;

-- smallint checks the integer range
CREATE TYPE smallint BASE integer
    ENCODE CASE WHEN value BETWEEN -32768 AND 32767 THEN value
                ELSE RAISE(ABORT, 'integer out of range for smallint') END
    DECODE value;```
```
---
## `text_file:cli/manuals/custom-types.md:305:18`

```markdown
```sql
CREATE TYPE varchar(maxlen) BASE text
    ENCODE CASE WHEN length(value) <= maxlen THEN value
                ELSE RAISE(ABORT, 'value too long for varchar') END
    DECODE value;

CREATE TABLE t1(name varchar(10)) STRICT;
INSERT INTO t1 VALUES ('hello');      -- OK (length 5 <= 10)
INSERT INTO t1 VALUES ('toolongname'); -- Error: value too long for varchar```
```
---
## `text_file:cli/manuals/custom-types.md:322:19`

```markdown
```sql
CREATE TYPE my_uuid BASE text ENCODE uuid_blob(value) DECODE uuid_str(value);
CREATE TABLE t1(id my_uuid PRIMARY KEY, name TEXT) STRICT;
INSERT INTO t1 VALUES ('invalid-uuid', 'bad');
-- Error: NOT NULL constraint failed (uuid_blob returned NULL)```
```
---
## `text_file:cli/manuals/custom-types.md:333:20`

```markdown
```sql
-- ERROR: type mismatch in CHECK constraint (cents vs INTEGER)
CREATE TABLE t1(amount cents CHECK(amount < 50)) STRICT;

-- OK: CAST converts the literal to cents, both sides have the same type
CREATE TABLE t1(amount cents CHECK(amount < CAST(50 AS cents))) STRICT;```
```
---
## `text_file:cli/manuals/custom-types.md:343:21`

```markdown
```sql
-- ERROR: type mismatch (INTEGER vs TEXT)
CREATE TABLE t1(age INTEGER CHECK(age < 'old')) STRICT;

-- OK: same types
CREATE TABLE t1(age INTEGER CHECK(age >= 18)) STRICT;```
```
---
## `text_file:cli/manuals/custom-types.md:353:22`

```markdown
```sql
-- ERROR: cannot determine return type of length()
CREATE TABLE t1(name TEXT CHECK(length(name) < 10)) STRICT;

-- OK: CAST makes the type explicit
CREATE TABLE t1(name TEXT CHECK(CAST(length(name) AS INTEGER) < 10)) STRICT;```
```
---
## `text_file:cli/manuals/custom-types.md:365:23`

```markdown
```sql
CREATE TYPE uint BASE text
    ENCODE test_uint_encode(value)
    DECODE test_uint_decode(value);
CREATE TABLE t1(val uint) STRICT;
INSERT INTO t1 VALUES (NULL);
SELECT COALESCE(val, 'IS_NULL') FROM t1;
-- IS_NULL```
```
---
## `text_file:cli/manuals/custom-types.md:379:24`

```markdown
```sql
CREATE TYPE reversed BASE text
    ENCODE string_reverse(value)
    DECODE string_reverse(value);
SELECT CAST('hello' AS reversed);
-- olleh```
```
---
## `text_file:cli/manuals/custom-types.md:393:25`

```markdown
```sql
PRAGMA list_types;
-- type      | parent | encode                | decode                | default | operators
-- INTEGER   |        |                       |                       |         |
-- REAL      |        |                       |                       |         |
-- TEXT      |        |                       |                       |         |
-- BLOB      |        |                       |                       |         |
-- ANY       |        |                       |                       |         |
-- uint      | text   | test_uint_encode(...) | test_uint_decode(...) | 0       | +(uint) -> test_uint_add```
```
---
## `text_file:cli/manuals/custom-types.md:408:26`

```markdown
```sql
SELECT name, sql FROM sqlite_turso_types;```
```
---
## `text_file:cli/manuals/custom-types.md:40:1`

```markdown
```sql
DROP TYPE type_name;
DROP TYPE IF EXISTS type_name;```
```
---
## `text_file:cli/manuals/custom-types.md:416:27`

```markdown
```sql
CREATE TYPE uint BASE text
    ENCODE test_uint_encode(value)
    DECODE test_uint_decode(value);
CREATE TABLE t1(id INTEGER PRIMARY KEY) STRICT;
ALTER TABLE t1 ADD COLUMN val uint;
INSERT INTO t1 VALUES (1, 42);
SELECT id, val FROM t1;
-- 1|42```
```
---
## `text_file:cli/manuals/custom-types.md:53:2`

```markdown
```sql
CREATE TYPE passthrough BASE text ENCODE value DECODE value;
CREATE TABLE t1(val passthrough) STRICT;
INSERT INTO t1 VALUES ('hello');
SELECT val FROM t1;
-- hello```
```
---
## `text_file:cli/manuals/custom-types.md:65:3`

```markdown
```sql
CREATE TYPE reversed BASE text
    ENCODE string_reverse(value)
    DECODE string_reverse(value);
CREATE TABLE t1(val reversed) STRICT;
INSERT INTO t1 VALUES ('hello');
SELECT val FROM t1;
-- hello  (stored on disk as 'olleh')```
```
---
## `text_file:cli/manuals/custom-types.md:79:4`

```markdown
```sql
CREATE TYPE cents BASE integer ENCODE value * 100 DECODE value / 100;
CREATE TABLE prices(amount cents) STRICT;
INSERT INTO prices VALUES (42);
SELECT amount FROM prices;
-- 42  (stored on disk as 4200)```
```
---
## `text_file:cli/manuals/custom-types.md:91:5`

```markdown
```sql
CREATE TYPE jsontype BASE text ENCODE json(value) DECODE value;
CREATE TABLE t1(val jsontype) STRICT;
INSERT INTO t1 VALUES ('{"key": 1}');  -- OK
INSERT INTO t1 VALUES ('not json');    -- Error: malformed JSON```
```
---
## `text_file:cli/manuals/encryption.md:34:0`

```markdown
```bash
# For 32-byte key (256-bit) - use with aes256gcm, aegis256, etc.
openssl rand -hex 32

# For 16-byte key (128-bit) - use with aes128gcm, aegis128l, etc.
openssl rand -hex 16```
```
---
## `text_file:cli/manuals/encryption.md:55:1`

```markdown
```bash
tursodb --experimental-encryption database.db```
```
---
## `text_file:cli/manuals/encryption.md:60:2`

```markdown
```sql
PRAGMA cipher = 'aegis256';
PRAGMA hexkey = '2d7a30108d3eb3e45c90a732041fe54778bdcf707c76749fab7da335d1b39c1d';

-- Now create your tables and insert data
CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
INSERT INTO users VALUES (1, 'Alice');```
```
---
## `text_file:cli/manuals/encryption.md:73:3`

```markdown
```bash
tursodb --experimental-encryption "file:database.db?cipher=aegis256&hexkey=2d7a30108d3eb3e45c90a732041fe54778bdcf707c76749fab7da335d1b39c1d"```
```
---
## `text_file:cli/manuals/materialized-views.md:14:0`

```markdown
```bash
tursodb --experimental-views your_database.db```
```
---
## `text_file:cli/manuals/materialized-views.md:54:1`

```markdown
```sql
CREATE MATERIALIZED VIEW sales_summary AS
SELECT
    product_id,
    COUNT(*) as total_sales,
    SUM(amount) as revenue,
    AVG(amount) as avg_sale_amount
FROM sales
GROUP BY product_id;```
```
---
## `text_file:cli/manuals/materialized-views.md:67:2`

```markdown
```sql
SELECT * FROM sales_summary WHERE revenue > 10000;```
```
---
## `text_file:cli/manuals/mcp.md:16:0`

```markdown
```bash
/path/to/tursodb --mcp```
```
---
## `text_file:cli/manuals/mcp.md:33:1`

```markdown
```json
{
  "tool": "query",
  "arguments": {
    "sql": "SELECT * FROM users WHERE age > 21"
  }
}```
```
---
## `text_file:cli/manuals/mcp.md:49:2`

```markdown
```json
{
  "tool": "execute",
  "arguments": {
    "sql": "INSERT INTO users (name, age) VALUES ('Alice', 30)"
  }
}```
```
---
## `text_file:cli/manuals/mcp.md:62:3`

```markdown
```json
{
  "tool": "list_tables",
  "arguments": {}
}```
```
---
## `text_file:cli/manuals/mcp.md:76:4`

```markdown
```json
{
  "tool": "describe_table",
  "arguments": {
    "table": "users"
  }
}```
```
---
## `text_file:cli/manuals/mcp.md:91:5`

```markdown
```json
{
  "mcpServers": {
    "turso": {
      "command": "/path/to/tursodb",
      "args": ["--mcp"]
    }
  }
}```
```
---
## `text_file:cli/manuals/vector.md:117:4`

```markdown
```sql
-- Find all documents within distance threshold
WITH query AS (
    SELECT vector32('[0.15, 0.25, 0.35, 0.45]') AS query_vector
)
SELECT
    id,
    content,
    vector_distance_l2(embedding, query_vector) AS distance
FROM documents, query
WHERE vector_distance_l2(embedding, query_vector) < 0.5
ORDER BY distance;```
```
---
## `text_file:cli/manuals/vector.md:135:5`

```markdown
```sql
-- Extract and view vector data as JSON
SELECT id, vector_extract(embedding) AS vector_json
FROM documents
LIMIT 3;```
```
---
## `text_file:cli/manuals/vector.md:144:6`

```markdown
```sql
-- Concatenate two vectors
SELECT vector_concat(
    vector32('[1.0, 2.0]'),
    vector32('[3.0, 4.0]')
) AS concatenated;

-- Slice a vector (extract dimensions 2-4)
SELECT vector_slice(
    vector32('[1.0, 2.0, 3.0, 4.0, 5.0]'),
    2, 4
) AS sliced;```
```
---
## `text_file:cli/manuals/vector.md:162:7`

```markdown
```sql
-- 1. Create schema
CREATE TABLE articles (
    id INTEGER PRIMARY KEY,
    title TEXT,
    content TEXT,
    embedding BLOB
);

-- 2. Insert pre-computed embeddings
INSERT INTO articles VALUES
    (1, 'Database Fundamentals', 'An introduction to relational databases...',
     vector32('[0.12, -0.34, 0.56, ...]')),
    (2, 'Machine Learning Basics', 'Understanding neural networks and deep learning...',
     vector32('[0.23, 0.45, -0.67, ...]')),
    (3, 'Web Development Guide', 'Modern web applications with JavaScript...',
     vector32('[0.34, -0.12, 0.78, ...]'));

-- 3. Search for similar articles
WITH search_embedding AS (
    -- This would come from your embedding model for the search query
    SELECT vector32('[0.15, -0.30, 0.60, ...]') AS query_vec
)
SELECT
    a.id,
    a.title,
    vector_distance_cos(a.embedding, s.query_vec) AS similarity_score
FROM articles a, search_embedding s
ORDER BY similarity_score
LIMIT 10;```
```
---
## `text_file:cli/manuals/vector.md:27:0`

```markdown
```sql
-- Create a table with vector embeddings
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    content TEXT,
    embedding BLOB  -- Store vector as BLOB
);

-- Insert vectors using vector32() or vector64()
INSERT INTO documents VALUES
    (1, 'Introduction to databases', vector32('[0.1, 0.2, 0.3, 0.4]')),
    (2, 'SQL query optimization', vector32('[0.2, 0.1, 0.4, 0.3]')),
    (3, 'Vector similarity search', vector32('[0.4, 0.3, 0.2, 0.1]'));```
```
---
## `text_file:cli/manuals/vector.md:46:1`

```markdown
```sql
-- Example with 1536-dimensional embeddings (like OpenAI's ada-002)
CREATE TABLE embeddings (
    id INTEGER PRIMARY KEY,
    text TEXT,
    vector BLOB
);

-- Insert a 1536-dimensional vector
INSERT INTO embeddings VALUES
    (1, 'Sample text', vector32('[0.001, 0.002, ..., 0.1536]'));```
```
---
## `text_file:cli/manuals/vector.md:81:2`

```markdown
```sql
-- Find documents similar to a query vector
WITH query AS (
    SELECT vector32('[0.15, 0.25, 0.35, 0.45]') AS query_vector
)
SELECT
    id,
    content,
    vector_distance_l2(embedding, query_vector) AS distance
FROM documents, query
ORDER BY distance
LIMIT 5;```
```
---
## `text_file:cli/manuals/vector.md:99:3`

```markdown
```sql
-- Find semantically similar documents using cosine distance
WITH query AS (
    SELECT vector32('[0.15, 0.25, 0.35, 0.45]') AS query_vector
)
SELECT
    id,
    content,
    vector_distance_cos(embedding, query_vector) AS cosine_distance
FROM documents, query
ORDER BY cosine_distance
LIMIT 5;```
```
---
## `text_file:docs/agent-guides/async-io-model.md:120:7`

```markdown
```rust
fn bad_example(&mut self) -> Result<IOResult<()>> {
    self.counter += 1;  // Mutates state
    return_if_io!(something_that_might_yield());  // If yields, re-entry will increment again!
    Ok(IOResult::Done(()))
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:12:0`

```markdown
```rust
pub enum IOCompletions {
    Single(Completion),
}

#[must_use]
pub enum IOResult<T> {
    Done(T),      // Operation complete, here's the result
    IO(IOCompletions),  // Need I/O, call me again after completions finish
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:131:8`

```markdown
```rust
fn good_example(&mut self) -> Result<IOResult<()>> {
    return_if_io!(something_that_might_yield());
    self.counter += 1;  // Only reached once, after IO completes
    Ok(IOResult::Done(()))
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:140:9`

```markdown
```rust
enum State { Start, AfterIO }

fn good_example(&mut self) -> Result<IOResult<()>> {
    loop {
        match self.state {
            State::Start => {
                // Don't mutate shared state here
                self.state = State::AfterIO;
                return_if_io!(something_that_might_yield());
            }
            State::AfterIO => {
                self.counter += 1;  // Safe: only entered once
                return Ok(IOResult::Done(()));
            }
        }
    }
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:173:10`

```markdown
```rust
// Good: index is part of state, preserved across yields
enum ProcessState {
    Start,
    ProcessingItem { idx: usize, items: Vec<Item> },
    Done,
}

// Loop advances idx only when transitioning states
ProcessingItem { idx, items } => {
    return_if_io!(process_item(&items[idx]));
    if idx + 1 < items.len() {
        self.state = ProcessingItem { idx: idx + 1, items };
    } else {
        self.state = Done;
    }
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:30:1`

```markdown
```rust
pub struct Completion { /* ... */ }

impl Completion {
    pub fn finished(&self) -> bool;
    pub fn succeeded(&self) -> bool;
    pub fn get_error(&self) -> Option<CompletionError>;
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:42:2`

```markdown
```rust
let mut group = CompletionGroup::new(|_| {});

// Add individual completions
group.add(&completion1);
group.add(&completion2);

// Build into single completion that finishes when all complete
let combined = group.build();
io_yield_one!(combined);```
```
---
## `text_file:docs/agent-guides/async-io-model.md:64:3`

```markdown
```rust
let result = return_if_io!(some_io_operation());
// Only reaches here if operation returned Done```
```
---
## `text_file:docs/agent-guides/async-io-model.md:71:4`

```markdown
```rust
io_yield_one!(completion);  // Returns Ok(IOResult::IO(Single(completion)))```
```
---
## `text_file:docs/agent-guides/async-io-model.md:79:5`

```markdown
```rust
enum MyOperationState {
    Start,
    WaitingForRead { page: PageRef },
    Processing { data: Vec<u8> },
    Done,
}```
```
---
## `text_file:docs/agent-guides/async-io-model.md:90:6`

```markdown
```rust
fn my_operation(&mut self) -> Result<IOResult<Output>> {
    loop {
        match &mut self.state {
            MyOperationState::Start => {
                let (page, completion) = start_read();
                self.state = MyOperationState::WaitingForRead { page };
                io_yield_one!(completion);
            }
            MyOperationState::WaitingForRead { page } => {
                let data = page.get_contents();
                self.state = MyOperationState::Processing { data: data.to_vec() };
                // No yield, continue loop
            }
            MyOperationState::Processing { data } => {
                let result = process(data);
                self.state = MyOperationState::Done;
                return Ok(IOResult::Done(result));
            }
            MyOperationState::Done => unreachable!(),
        }
    }
}```
```
---
## `text_file:docs/agent-guides/code-quality.md:32:0`

```markdown
```rust
// Good: documents the invariant
let value = option.expect("value must be set in Init phase");```
```
---
## `text_file:docs/agent-guides/code-quality.md:38:1`

```markdown
```rust
// Good: proper error handling
let Some(value) = option else {
    return Err(LimboError::InvalidArgument("value not provided".into()));
};```
```
---
## `text_file:docs/agent-guides/code-quality.md:52:2`

```markdown
```rust
if condition {
    // happy path
} else {
    // "shouldn't happen" - silently ignored
}```
```
---
## `text_file:docs/agent-guides/code-quality.md:61:3`

```markdown
```rust
// If only one branch should ever be hit:
assert!(condition, "invariant violated: ...");
// OR
return Err(LimboError::InternalError("unexpected state".into()));
// OR
unreachable!("impossible state: ...");```
```
---
## `text_file:docs/agent-guides/debugging.md:22:0`

```markdown
```bash
# SQLite
sqlite3 :memory: "EXPLAIN SELECT 1 + 1;"

# Turso
cargo run --bin tursodb :memory: "EXPLAIN SELECT 1 + 1;"```
```
---
## `text_file:docs/agent-guides/debugging.md:32:1`

```markdown
```bash
cargo run --bin tursodb :memory: 'SELECT * FROM foo;'
cargo run --bin tursodb :memory: 'EXPLAIN SELECT * FROM foo;'```
```
---
## `text_file:docs/agent-guides/debugging.md:39:2`

```markdown
```bash
# Trace core during tests
RUST_LOG=none,turso_core=trace make test

# Output goes to testing/test.log
# Warning: can be megabytes per test run```
```
---
## `text_file:docs/agent-guides/debugging.md:51:3`

```markdown
```bash
rustup toolchain install nightly
rustup override set nightly
cargo run -Zbuild-std --target x86_64-unknown-linux-gnu \
  -p turso_stress -- --vfs syscall --nr-threads 4 --nr-iterations 1000```
```
---
## `text_file:docs/agent-guides/debugging.md:62:4`

```markdown
```bash
# Simulator
RUST_LOG=limbo_sim=debug cargo run --bin limbo_sim -- -s <seed>

# Whopper (concurrent DST)
SEED=1234 ./testing/concurrent-simulator/bin/run```
```
---
## `text_file:docs/agent-guides/mvcc.md:14:0`

```markdown
```sql
PRAGMA journal_mode = 'mvcc';```
```
---
## `text_file:docs/agent-guides/mvcc.md:69:1`

```markdown
```sql
PRAGMA mvcc_checkpoint_threshold = <pages>;```
```
---
## `text_file:docs/agent-guides/mvcc.md:87:2`

```markdown
```bash
# Run MVCC-specific tests
cargo test mvcc

# TCL tests with MVCC
make test-mvcc```
```
---
## `text_file:docs/agent-guides/mvcc.md:97:3`

```markdown
```rust
#[turso_macros::test(mvcc)]
fn test_something() {
    // runs with MVCC enabled
}```
```
---
## `text_file:docs/agent-guides/storage-format.md:127:0`

```markdown
```bash
# Integrity check
cargo run --bin tursodb test.db "PRAGMA integrity_check;"

# Page count
cargo run --bin tursodb test.db "PRAGMA page_count;"

# Freelist info
cargo run --bin tursodb test.db "PRAGMA freelist_count;"```
```
---
## `text_file:docs/agent-guides/testing.md:21:0`

```markdown
```bash
# Main test suite (TCL compat, sqlite3 compat, Python wrappers)
make test

# Single TCL test
make test-single TEST=select.test

# SQL test runner
make -C testing/runner run-cli

# Rust unit/integration tests (full workspace)
cargo test```
```
---
## `text_file:docs/agent-guides/testing.md:38:1`

```markdown
```sql
@database :memory:

@query
SELECT 1 + 1;
@expected
2```
```
---
## `text_file:docs/agent-guides/testing.md:57:3`

```markdown
```rust
// tests/integration/test_foo.rs
#[test]
fn test_something() {
    let conn = Connection::open_in_memory().unwrap();
    // ...
}```
```
---
## `text_file:docs/agent-guides/testing.md:80:4`

```markdown
```bash
RUST_LOG=none,turso_core=trace make test```
```
---
## `text_file:docs/contributing/contributing_functions.md:57:0`

```markdown
```bash
> sqlite3

sqlite> explain select date('now');
addr  opcode         p1    p2    p3    p4             p5  comment
----  -------------  ----  ----  ----  -------------  --  -------------
0     Init           0     6     0                    0   Start at 6
1     Once           0     3     0                    0
2     Function       0     0     2     date(-1)       0   r[2]=func()
3     Copy           2     1     0                    0   r[1]=r[2]
4     ResultRow      1     1     0                    0   output=r[1]
5     Halt           0     0     0                    0
6     Goto           0     1     0                    0```
```
---
## `text_file:docs/contributing/contributing_functions.md:73:1`

```markdown
```bash
# created a sqlite database file database.db
# or cargo run to use the memory mode if it is already available.
> cargo run database.db

Enter ".help" for usage hints.
limbo> explain select date('now');
Parse error: unknown function date```
```
---
## `text_file:docs/fts.md:120:2`

```markdown
```sql
-- Title matches are 2x more important than body matches
CREATE INDEX idx_articles ON articles USING fts (title, body)
WITH (weights = 'title=2.0,body=1.0');

-- Combined with tokenizer
CREATE INDEX idx_docs ON docs USING fts (name, description)
WITH (tokenizer = 'simple', weights = 'name=3.0,description=1.0');```
```
---
## `text_file:docs/fts.md:138:3`

```markdown
```sql
-- Default tokenizer: "Hello World" → ["hello", "world"]
-- Searches for "hello" or "HELLO" will match

-- Raw tokenizer: "user-123" → ["user-123"]
-- Only exact match "user-123" will work, "user" won't match

-- Ngram tokenizer: "iPhone" → ["iP", "iPh", "Ph", "Pho", "ho", "hon", "on", "one", "ne"]
-- Search for "Pho" will match documents containing "iPhone"```
```
---
## `text_file:docs/fts.md:161:4`

```markdown
```sql
-- Get scores for matching documents, ordered by relevance
SELECT fts_score(title, body, 'database') as score, id, title
FROM articles
ORDER BY score DESC
LIMIT 10;

-- Simple match filter
SELECT id, title FROM articles WHERE fts_match(body, 'science') LIMIT 10;```
```
---
## `text_file:docs/fts.md:176:5`

```markdown
```sql
-- Basic highlighting (single column)
SELECT fts_highlight('Learn about database optimization', '<b>', '</b>', 'database');
-- Returns: "Learn about <b>database</b> optimization"

-- Multiple columns - text is concatenated with spaces
SELECT fts_highlight(title, body, '<mark>', '</mark>', 'database') as highlighted
FROM articles
WHERE fts_match(title, body, 'database');
-- If title='Database Design' and body='Learn about optimization',
-- Returns: "<mark>Database</mark> Design Learn about optimization"

-- Use with FTS queries to highlight matched content
SELECT
    id,
    title,
    fts_highlight(body, '<mark>', '</mark>', 'database') as highlighted_body
FROM articles
WHERE fts_match(title, body, 'database')
ORDER BY fts_score(title, body, 'database') DESC;

-- Multiple terms are highlighted
SELECT fts_highlight('The quick brown fox', '<em>', '</em>', 'quick fox');
-- Returns: "The <em>quick</em> brown <em>fox</em>"```
```
---
## `text_file:docs/fts.md:233:6`

```markdown
```sql
-- Complex query with extra columns and WHERE conditions
SELECT id, author, title, category, views, fts_score(title, body, 'Rust') as score
FROM articles
WHERE fts_match(title, body, 'Rust')
  AND category = 'tech'
  AND views > 100
ORDER BY score DESC;

-- ORDER BY non-score column
SELECT id, title FROM docs WHERE fts_match(title, body, 'Rust') ORDER BY created_at DESC;```
```
---
## `text_file:docs/fts.md:271:7`

```markdown
```sql
-- These trigger automatic FTS index updates
INSERT INTO articles VALUES (1, 'Title', 'Body text');
UPDATE articles SET body = 'New body' WHERE id = 1;
DELETE FROM articles WHERE id = 1;```
```
---
## `text_file:docs/fts.md:296:8`

```markdown
```sql
CREATE TABLE fts_dir_{idx_id} ( 
      path TEXT NOT NULL, 
      chunk_no INTEGER NOT NULL,
      bytes BLOB NOT NULL
    );
    ```
    
- **Index:**
    
    ```sql
    CREATE INDEX IF NOT EXISTS idx_name ON table_name USING backing_btree (path, chunk_no, bytes)
    ```
 

Use `backing_btree` to create a BTree that stores all columns without rowid indirection
This allows direct cursor access with the exact key structure. This way we can use an index cursor to `SeekGE` (path, chunk_no) where chunk_no is just computed from the offset requested by `read_bytes` on the file handle.

# Current Architecture: HybridBTreeDirectory

The architecture uses a hybrid approach that balances memory usage and performance:```
```
---
## `text_file:docs/fts.md:366:9`

```markdown
```rust
impl FileHandle for LazyFileHandle {
    fn read_bytes(&self, range: Range<usize>) -> io::Result<OwnedBytes> {
        // 1. Check hot cache
        // 2. Calculate required chunks from byte range
        // 3. For each chunk: check LRU cache, or blocking BTree fetch
        // 4. Assemble and return result
    }
}```
```
---
## `text_file:docs/fts.md:381:10`

```markdown
```rust
pub const DEFAULT_CHUNK_CACHE_BYTES: usize = 128 * 1024 * 1024; // 128MB
pub const DEFAULT_HOT_CACHE_BYTES: usize = 64 * 1024 * 1024;    // 64MB```
```
---
## `text_file:docs/fts.md:408:11`

```markdown
```rust
DEFAULT_MEMORY_BUDGET_BYTES  = 64 MB   // Tantivy IndexWriter memory budget
DEFAULT_CHUNK_SIZE           = 1 MB    // BTree blob chunk size
DEFAULT_HOT_CACHE_BYTES      = 64 MB   // Bounded LRU cache for metadata/term dicts
DEFAULT_CHUNK_CACHE_BYTES    = 128 MB  // Bounded LRU cache for segment chunks
BATCH_COMMIT_SIZE            = 1000    // Documents per Tantivy commit```
```
---
## `text_file:docs/fts.md:426:12`

```markdown
```sql
-- Optimize a specific FTS index
OPTIMIZE INDEX fts_articles;

-- Optimize all FTS indexes in the database
OPTIMIZE INDEX;```
```
---
## `text_file:docs/fts.md:89:0`

```markdown
```sql
CREATE INDEX idx_posts
ON posts USING fts (title, body);```
```
---
## `text_file:docs/fts.md:98:1`

```markdown
```sql
-- Use raw tokenizer for exact-match fields (IDs, tags)
CREATE INDEX idx_tags ON articles USING fts (tag) WITH (tokenizer = 'raw');

-- Use ngram tokenizer for autocomplete/substring matching
CREATE INDEX idx_products ON products USING fts (name) WITH (tokenizer = 'ngram');```
```
---
## `text_file:docs/manual.md:1001:55`

```markdown
```sql
-- Optimize a specific FTS index
OPTIMIZE INDEX idx_articles;

-- Optimize all FTS indexes
OPTIMIZE INDEX;```
```
---
## `text_file:docs/manual.md:1013:56`

```markdown
```sql
-- Create a documents table
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    title TEXT,
    content TEXT,
    category TEXT
);

-- Create FTS index with weighted fields
CREATE INDEX fts_docs ON documents USING fts (title, content)
WITH (weights = 'title=2.0,content=1.0');

-- Insert documents
INSERT INTO documents VALUES
    (1, 'Introduction to SQL', 'Learn SQL basics and queries', 'tutorial'),
    (2, 'Advanced SQL Techniques', 'Complex joins and optimization', 'tutorial'),
    (3, 'Database Design', 'Schema design best practices', 'architecture');

-- Search with relevance ranking
SELECT
    id,
    title,
    fts_score(title, content, 'SQL') as score,
    fts_highlight(content, '<b>', '</b>', 'SQL') as snippet
FROM documents
WHERE fts_match(title, content, 'SQL')
ORDER BY score DESC;```
```
---
## `text_file:docs/manual.md:1056:57`

```markdown
```sql
PRAGMA capture_data_changes_conn('<mode>[,custom_cdc_table]');```
```
---
## `text_file:docs/manual.md:1166:59`

```markdown
```sql
CREATE INDEX t_idx ON t USING index_method_name (column1, column2);```
```
---
## `text_file:docs/manual.md:1172:60`

```markdown
```sql
CREATE INDEX t_idx ON t USING index_method_name (c) WITH (a = 1, b = 1.2, c = 'text', d = x'deadbeef');```
```
---
## `text_file:docs/manual.md:1192:61`

```markdown
```sql
SELECT vector_distance_jaccard(embedding, ?) AS distance FROM documents ORDER BY distance LIMIT ?;```
```
---
## `text_file:docs/manual.md:1200:62`

```markdown
```sql
SELECT id, content, created_at FROM documents ORDER BY vector_distance_jaccard(embedding, ?) LIMIT 10;```
```
---
## `text_file:docs/manual.md:1208:63`

```markdown
```sql
SELECT id, content, created_at FROM documents WHERE user = ? ORDER BY vector_distance_jaccard(embedding, ?) LIMIT 10;```
```
---
## `text_file:docs/manual.md:1371:65`

```markdown
```rust
if !completion.is_completed {
    return StepResult::IO;
  }
  ```

This allows us to be flexible in places where we do not have the state machines in place to correctly return the Completion. Thus, we can block in certain places to avoid bigger refactorings, which opens up the opportunity for such refactorings in separate PRs.

To know if a function does any sort of I/O we just have to look at the function signature. If it returns `Completion`, `Vec<Completion>` or `IOResult`, then it does I/O.

The `IOResult` struct looks as follows:
  ```rust
  pub enum IOCompletions {
    Single(Completion),
  }

  #[must_use]
  pub enum IOResult<T> {
    Done(T),
    IO(IOCompletions),
  }
  ```

To combine multiple completions, use `CompletionGroup`:
  ```rust
  let mut group = CompletionGroup::new(|_| {});
  group.add(&completion1);
  group.add(&completion2);
  let combined = group.build();  // Single completion that waits for all
  ```

This implies that when a function returns an `IOResult`, it must be called again until it returns an `IOResult::Done` variant. This works similarly to how `Future`s are polled in rust. When you receive a `Poll::Ready(None)`, it means that the future stopped it's execution. In a similar vein, if we receive `IOResult::Done`, the function/state machine has reached the end of it's execution. `IOCompletions` is here to signal that, if we are executing any I/O operation, that we need to propagate the completions that are generated from it. This design forces us to handle the fact that a function is asynchronous in nature. This is essentially [function coloring](https://www.tedinski.com/2018/11/13/function-coloring.html), but done at the application level instead of the compiler level.

### Encryption

#### Goals

- Per-page encryption as an opt-in feature, so users don't have to compile/load the encryption extension
- All pages in db and WAL file to be encrypted on disk
- Least performance overhead as possible

#### Design

1. We do encryption at the page level, i.e., each page is encrypted and decrypted individually.
2. At db creation, we take key and cipher scheme information. We store the scheme information (also version) in the db file itself.
3. The key is not stored anywhere. So each connection should carry an encryption key. Trying to open a db with an invalid or empty key should return an error.
4. We generate a new randomized, cryptographically safe nonce every time we write a page.
5. We store the authentication tag and the nonce in the page itself.
6. We can support different cipher algorithms: AES, ChachaPoly, AEGIS, etc.
7. We can support key rotation. But rekeying would require writing the entire database.
8. We should also add import/export functionality to the CLI for better DX and compatibility with SQLite.

#### Metadata management

We store the nonce and tag (or the verification bits) in the page itself. During decryption, we will load these to decrypt and verify the data.

Example: Assume the page size is 4096 bytes and we use AEGIS 256. So we reserve the last 48 bytes
for the nonce (32 bytes) and tag (16 bytes).```
```
---
## `text_file:docs/manual.md:235:2`

```markdown
```sql
ALTER TABLE old_name RENAME TO new_name

ALTER TABLE table_name ADD COLUMN column_name [ column_type ]

ALTER TABLE table_name DROP COLUMN column_name```
```
---
## `text_file:docs/manual.md:261:4`

```markdown
```sql
BEGIN [ transaction_mode ] [ TRANSACTION ]```
```
---
## `text_file:docs/manual.md:281:5`

```markdown
```sql
COMMIT [ TRANSACTION ]```
```
---
## `text_file:docs/manual.md:296:6`

```markdown
```sql
CREATE INDEX [ index_name ] ON table_name ( column_name )```
```
---
## `text_file:docs/manual.md:311:7`

```markdown
```sql
CREATE TABLE table_name ( column_name [ column_type ], ... )```
```
---
## `text_file:docs/manual.md:328:9`

```markdown
```sql
DELETE FROM table_name [ WHERE expression ]```
```
---
## `text_file:docs/manual.md:359:13`

```markdown
```sql
END [ TRANSACTION ]```
```
---
## `text_file:docs/manual.md:371:14`

```markdown
```sql
INSERT INTO table_name [ ( column_name, ... ) ] VALUES ( value, ... ) [, ( value, ... ) ...]```
```
---
## `text_file:docs/manual.md:393:15`

```markdown
```sql
ROLLBACK [ TRANSACTION ]```
```
---
## `text_file:docs/manual.md:401:16`

```markdown
```sql
SELECT expression
    [ FROM table-or-subquery ]
    [ WHERE condition ]
    [ GROUP BY expression ]```
```
---
## `text_file:docs/manual.md:433:18`

```markdown
```sql
UPDATE table_name SET column_name = value [WHERE expression]```
```
---
## `text_file:docs/manual.md:510:22`

```markdown
```c
int sqlite3_open(const char *filename, sqlite3 **db_out);
int sqlite3_open_v2(const char *filename, sqlite3 **db_out, int _flags, const char *_z_vfs);```
```
---
## `text_file:docs/manual.md:521:23`

```markdown
```c
int sqlite3_prepare_v2(sqlite3 *db, const char *sql, int _len, sqlite3_stmt **out_stmt, const char **_tail);```
```
---
## `text_file:docs/manual.md:531:24`

```markdown
```c
int sqlite3_step(sqlite3_stmt *stmt);```
```
---
## `text_file:docs/manual.md:541:25`

```markdown
```c
int sqlite3_column_type(sqlite3_stmt *_stmt, int _idx);
int sqlite3_column_count(sqlite3_stmt *_stmt);
const char *sqlite3_column_decltype(sqlite3_stmt *_stmt, int _idx);
const char *sqlite3_column_name(sqlite3_stmt *_stmt, int _idx);
int64_t sqlite3_column_int64(sqlite3_stmt *_stmt, int _idx);
double sqlite3_column_double(sqlite3_stmt *_stmt, int _idx);
const void *sqlite3_column_blob(sqlite3_stmt *_stmt, int _idx);
int sqlite3_column_bytes(sqlite3_stmt *_stmt, int _idx);
const unsigned char *sqlite3_column_text(sqlite3_stmt *stmt, int idx);```
```
---
## `text_file:docs/manual.md:561:26`

```markdown
```c
int libsql_wal_frame_count(sqlite3 *db, uint32_t *p_frame_count);```
```
---
## `text_file:docs/manual.md:601:27`

```markdown
```sql
PRAGMA journal_mode;```
```
---
## `text_file:docs/manual.md:607:28`

```markdown
```sql
PRAGMA journal_mode = wal;```
```
---
## `text_file:docs/manual.md:613:29`

```markdown
```sql
PRAGMA journal_mode = mvcc;```
```
---
## `text_file:docs/manual.md:646:31`

```markdown
```shell
$ openssl rand -hex 32
2d7a30108d3eb3e45c90a732041fe54778bdcf707c76749fab7da335d1b39c1d```
```
---
## `text_file:docs/manual.md:653:32`

```markdown
```shell
$ cargo run -- --experimental-encryption database.db

PRAGMA cipher = 'aegis256'; -- or 'aes256gcm'
PRAGMA hexkey = '2d7a30108d3eb3e45c90a732041fe54778bdcf707c76749fab7da335d1b39c1d';```
```
---
## `text_file:docs/manual.md:660:33`

```markdown
```shell
$ cargo run -- --experimental-encryption \
"file:database.db?cipher=aegis256&hexkey=2d7a30108d3eb3e45c90a732041fe54778bdcf707c76749fab7da335d1b39c1d"```
```
---
## `text_file:docs/manual.md:668:34`

```markdown
```shell
$ cargo run -- --experimental-encryption \
   "file:database.db?cipher=aegis256hexkey=2d7a30108d3eb3e45c90a732041fe54778bdcf707c76749fab7da335d1b39c1d"```
```
---
## `text_file:docs/manual.md:719:35`

```markdown
```sql
SELECT vector32('[1.0, 2.0, 3.0]');```
```
---
## `text_file:docs/manual.md:727:36`

```markdown
```sql
SELECT vector32_sparse('[0.0, 1.5, 0.0, 2.3, 0.0]');```
```
---
## `text_file:docs/manual.md:735:37`

```markdown
```sql
SELECT vector64('[1.0, 2.0, 3.0]');```
```
---
## `text_file:docs/manual.md:743:38`

```markdown
```sql
SELECT vector8('[1.0, 2.0, 3.0, 4.0]');```
```
---
## `text_file:docs/manual.md:751:39`

```markdown
```sql
SELECT vector_extract(vector1bit('[1, -1, 1, 1, -1, 0, 0.5]'));
-- Returns: [1,-1,1,1,-1,-1,1]```
```
---
## `text_file:docs/manual.md:760:40`

```markdown
```sql
SELECT vector_extract(embedding) FROM documents;```
```
---
## `text_file:docs/manual.md:776:41`

```markdown
```sql
SELECT name, vector_distance_cos(embedding, vector32('[0.1, 0.5, 0.3]')) AS distance
FROM documents
ORDER BY distance
LIMIT 10;```
```
---
## `text_file:docs/manual.md:792:42`

```markdown
```sql
SELECT name, vector_distance_l2(embedding, vector32('[0.1, 0.5, 0.3]')) AS distance
FROM documents
ORDER BY distance
LIMIT 10;```
```
---
## `text_file:docs/manual.md:807:43`

```markdown
```sql
SELECT name, vector_distance_dot(embedding, vector32('[0.1, 0.5, 0.3]')) AS distance
FROM documents
ORDER BY distance
LIMIT 10;```
```
---
## `text_file:docs/manual.md:824:44`

```markdown
```sql
SELECT name, vector_distance_jaccard(sparse_embedding, vector32_sparse('[0.0, 1.0, 0.0, 2.0]')) AS distance
FROM documents
ORDER BY distance
LIMIT 10;```
```
---
## `text_file:docs/manual.md:837:45`

```markdown
```sql
SELECT vector_concat(vector32('[1.0, 2.0]'), vector32('[3.0, 4.0]'));
-- Results in a 4-dimensional vector: [1.0, 2.0, 3.0, 4.0]```
```
---
## `text_file:docs/manual.md:846:46`

```markdown
```sql
SELECT vector_slice(vector32('[1.0, 2.0, 3.0, 4.0, 5.0]'), 1, 4);
-- Results in: [2.0, 3.0, 4.0]```
```
---
## `text_file:docs/manual.md:855:47`

```markdown
```sql
-- Create a table for documents with embeddings
CREATE TABLE documents (
    id INTEGER PRIMARY KEY,
    name TEXT,
    content TEXT,
    embedding BLOB
);

-- Insert documents with precomputed embeddings
INSERT INTO documents (name, content, embedding) VALUES
    ('Doc 1', 'Machine learning basics', vector32('[0.2, 0.5, 0.1, 0.8]')),
    ('Doc 2', 'Database fundamentals', vector32('[0.1, 0.3, 0.9, 0.2]')),
    ('Doc 3', 'Neural networks guide', vector32('[0.3, 0.6, 0.2, 0.7]'));

-- Find documents similar to a query embedding
SELECT
    name,
    content,
    vector_distance_cos(embedding, vector32('[0.25, 0.55, 0.15, 0.75]')) AS similarity
FROM documents
ORDER BY similarity
LIMIT 5;```
```
---
## `text_file:docs/manual.md:890:48`

```markdown
```sql
CREATE INDEX idx_articles ON articles USING fts (title, body);```
```
---
## `text_file:docs/manual.md:900:49`

```markdown
```sql
-- Use ngram tokenizer for autocomplete/substring matching
CREATE INDEX idx_products ON products USING fts (name) WITH (tokenizer = 'ngram');

-- Use raw tokenizer for exact-match fields
CREATE INDEX idx_tags ON articles USING fts (tag) WITH (tokenizer = 'raw');```
```
---
## `text_file:docs/manual.md:941:51`

```markdown
```sql
SELECT id, title FROM articles WHERE fts_match(title, body, 'database');```
```
---
## `text_file:docs/manual.md:949:52`

```markdown
```sql
SELECT fts_score(title, body, 'database') as score, id, title
FROM articles
WHERE fts_match(title, body, 'database')
ORDER BY score DESC
LIMIT 10;```
```
---
## `text_file:docs/manual.md:961:53`

```markdown
```sql
SELECT fts_highlight(body, '<mark>', '</mark>', 'database') as highlighted
FROM articles
WHERE fts_match(title, body, 'database');
-- Returns: "Learn about <mark>database</mark> optimization"```
```
---
## `text_file:docs/manual.md:988:54`

```markdown
```sql
SELECT id, title, fts_score(title, body, 'Rust') as score
FROM articles
WHERE fts_match(title, body, 'Rust')
  AND category = 'tech'
  AND published = 1
ORDER BY score DESC;```
```
---
## `text_file:docs/testing.md:112:4`

```markdown
```bash
RUST_LOG=limbo_sim=debug cargo run --bin limbo_sim```
```
---
## `text_file:docs/testing.md:12:0`

```markdown
```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    first_name TEXT,
    last_name TEXT,
    email TEXT,
    phone_number TEXT,
    address TEXT,
    city TEXT,
    state TEXT,
    zipcode TEXT,
    age INTEGER
);
CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT,
    price REAL
);
CREATE INDEX age_idx ON users (age);```
```
---
## `text_file:docs/testing.md:41:1`

```markdown
```python
from cli_tests.common import TestTursoShell

def test_uuid():
    limbo = TestTursoShell()
    limbo.run_test_fn("SELECT uuid4_str();", lambda res: len(res) == 36)
    limbo.quit()```
```
---
## `text_file:docs/testing.md:93:3`

```markdown
```sql
-- begin testing 'Select-Select-Optimizer'
-- ASSUME table marvelous_ideal exists;
SELECT ((devoted_ahmed = -9142609771.541502 AND loving_wicker = -1246708244.164486)) FROM marvelous_ideal WHERE TRUE;
SELECT * FROM marvelous_ideal WHERE (devoted_ahmed = -9142609771.541502 AND loving_wicker = -1246708244.164486);
-- ASSERT select queries should return the same amount of results;
-- end testing 'Select-Select-Optimizer'```
```
---
## `text_file:packages/turso-serverless/AGENT.md:121:2`

```markdown
```bash
npm test  # Runs all integration tests
npm run build  # TypeScript compilation```
```
---
## `text_file:packages/turso-serverless/AGENT.md:37:0`

```markdown
```typescript
import { connect } from "@tursodatabase/serverless";

const client = connect({ url, authToken });
const stmt = client.prepare("SELECT * FROM users WHERE id = ?", [123]);

// Three execution modes:
const row = await stmt.get();        // First row or null
const rows = await stmt.all();       // All rows as array  
for await (const row of stmt.iterate()) { ... } // Streaming iterator```
```
---
## `text_file:packages/turso-serverless/AGENT.md:78:1`

```markdown
```typescript
// ✅ Supported
const client = createClient({ url, authToken });
await client.execute(sql, args);
await client.batch(statements);

// ✅ Supported: remote encryption key for encrypted Turso Cloud databases
createClient({ url, authToken, remoteEncryptionKey: "base64-encoded-key" });

// ❌ Unsupported (throws LibsqlError)
createClient({ url, authToken, encryptionKey: "..." }); // Validation error
await client.transaction(); // Not implemented
await client.sync(); // Not supported for remote```
```
---
## `text_file:testing/runner/docs/adding-backends.md:20:0`

```markdown
```rust
#[async_trait]
pub trait SqlBackend: Send + Sync {
    /// Name of this backend (for filtering and display)
    fn name(&self) -> &str;

    /// Create a new isolated database instance
    async fn create_database(&self, config: &DatabaseConfig)
        -> Result<Box<dyn DatabaseInstance>, BackendError>;
}```
```
---
## `text_file:testing/runner/docs/adding-backends.md:244:5`

```markdown
```python
#!/usr/bin/env python3
import sys
import json
import turso  # or whatever the SDK module is called

def main():
    db_path = sys.argv[1]
    readonly = "--readonly" in sys.argv

    # Connect to database
    conn = turso.connect(db_path, readonly=readonly)

    # Process commands from stdin
    for line in sys.stdin:
        try:
            cmd = json.loads(line.strip())

            if cmd["type"] == "close":
                conn.close()
                break

            if cmd["type"] == "execute":
                sql = cmd["sql"]
                try:
                    cursor = conn.execute(sql)
                    rows = [[str(col) for col in row] for row in cursor.fetchall()]
                    print(json.dumps({"rows": rows}))
                except Exception as e:
                    print(json.dumps({"error": str(e)}))

            sys.stdout.flush()

        except Exception as e:
            print(json.dumps({"error": f"bridge error: {e}"}))
            sys.stdout.flush()

if __name__ == "__main__":
    main()```
```
---
## `text_file:testing/runner/docs/adding-backends.md:289:6`

```markdown
```rust
pub mod cli;
pub mod python;  // Add this

pub use cli::CliBackend;
pub use python::PythonBackend;  // Add this```
```
---
## `text_file:testing/runner/docs/adding-backends.md:301:7`

```markdown
```rust
#[derive(Subcommand)]
enum Commands {
    Run {
        // ...existing options...

        /// Backend to use (cli, python)
        #[arg(long, default_value = "cli")]
        backend: String,

        /// Path to Python executable (for python backend)
        #[arg(long)]
        python: Option<PathBuf>,
    },
}```
```
---
## `text_file:testing/runner/docs/adding-backends.md:34:1`

```markdown
```rust
#[async_trait]
pub trait DatabaseInstance: Send + Sync {
    /// Execute SQL and return results
    async fn execute(&mut self, sql: &str) -> Result<QueryResult, BackendError>;

    /// Close and cleanup the database
    async fn close(self: Box<Self>) -> Result<(), BackendError>;
}```
```
---
## `text_file:testing/runner/docs/adding-backends.md:365:8`

```markdown
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_memory_database() {
        let backend = YourBackend::new();
        let config = DatabaseConfig {
            location: DatabaseLocation::Memory,
            readonly: false,
        };

        let db = backend.create_database(&config).await;
        assert!(db.is_ok());
    }

    #[tokio::test]
    async fn test_execute_simple_query() {
        let backend = YourBackend::new();
        let config = DatabaseConfig {
            location: DatabaseLocation::Memory,
            readonly: false,
        };

        let mut db = backend.create_database(&config).await.unwrap();
        let result = db.execute("SELECT 1").await.unwrap();

        assert!(!result.is_error());
        assert_eq!(result.rows, vec![vec!["1".to_string()]]);
    }
}```
```
---
## `text_file:testing/runner/docs/adding-backends.md:403:9`

```markdown
```bash
test-runner run tests/ --backend python --python /usr/bin/python3```
```
---
## `text_file:testing/runner/docs/adding-backends.md:47:2`

```markdown
```rust
pub struct QueryResult {
    /// Rows returned, each row is a vector of string-formatted columns
    pub rows: Vec<Vec<String>>,
    /// Error message if the query failed
    pub error: Option<String>,
}

impl QueryResult {
    pub fn success(rows: Vec<Vec<String>>) -> Self;
    pub fn error(message: impl Into<String>) -> Self;
    pub fn is_error(&self) -> bool;
}```
```
---
## `text_file:testing/runner/docs/adding-backends.md:64:3`

```markdown
```rust
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("failed to create database: {0}")]
    CreateDatabase(String),

    #[error("failed to execute query: {0}")]
    Execute(String),

    #[error("failed to close database: {0}")]
    Close(String),

    #[error("backend not available: {0}")]
    NotAvailable(String),

    #[error("query timed out after {0:?}")]
    Timeout(Duration),
}```
```
---
## `text_file:testing/runner/docs/adding-backends.md:92:4`

```markdown
```rust
use super::{BackendError, DatabaseInstance, QueryResult, SqlBackend};
use crate::parser::ast::{DatabaseConfig, DatabaseLocation};
use async_trait::async_trait;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tempfile::NamedTempFile;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::time::timeout;

/// Python SDK backend
pub struct PythonBackend {
    /// Path to python executable
    python_path: PathBuf,
    /// Path to the bridge script
    bridge_script: PathBuf,
    /// Query timeout
    timeout: Duration,
}

impl PythonBackend {
    pub fn new(python_path: impl Into<PathBuf>, bridge_script: impl Into<PathBuf>) -> Self {
        Self {
            python_path: python_path.into(),
            bridge_script: bridge_script.into(),
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[async_trait]
impl SqlBackend for PythonBackend {
    fn name(&self) -> &str {
        "python"
    }

    async fn create_database(&self, config: &DatabaseConfig)
        -> Result<Box<dyn DatabaseInstance>, BackendError>
    {
        let (db_path, temp_file) = match &config.location {
            DatabaseLocation::Memory => (":memory:".to_string(), None),
            DatabaseLocation::TempFile => {
                let temp = NamedTempFile::new()
                    .map_err(|e| BackendError::CreateDatabase(e.to_string()))?;
                let path = temp.path().to_string_lossy().to_string();
                (path, Some(temp))
            }
            DatabaseLocation::Path(path) => (path.to_string_lossy().to_string(), None),
        };

        // Spawn Python process with the bridge script
        let mut cmd = Command::new(&self.python_path);
        cmd.arg(&self.bridge_script);
        cmd.arg(&db_path);

        if config.readonly {
            cmd.arg("--readonly");
        }

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| BackendError::CreateDatabase(format!("failed to spawn python: {}", e)))?;

        Ok(Box::new(PythonDatabaseInstance {
            child,
            timeout: self.timeout,
            _temp_file: temp_file,
        }))
    }
}

pub struct PythonDatabaseInstance {
    child: Child,
    timeout: Duration,
    _temp_file: Option<NamedTempFile>,
}

#[async_trait]
impl DatabaseInstance for PythonDatabaseInstance {
    async fn execute(&mut self, sql: &str) -> Result<QueryResult, BackendError> {
        // Send SQL as JSON command
        let request = serde_json::json!({
            "type": "execute",
            "sql": sql
        });

        let stdin = self.child.stdin.as_mut()
            .ok_or_else(|| BackendError::Execute("stdin not available".to_string()))?;

        stdin.write_all(request.to_string().as_bytes()).await
            .map_err(|e| BackendError::Execute(e.to_string()))?;
        stdin.write_all(b"\n").await
            .map_err(|e| BackendError::Execute(e.to_string()))?;

        // Read response
        let stdout = self.child.stdout.as_mut()
            .ok_or_else(|| BackendError::Execute("stdout not available".to_string()))?;

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();

        let result = timeout(self.timeout, reader.read_line(&mut line))
            .await
            .map_err(|_| BackendError::Timeout(self.timeout))?
            .map_err(|e| BackendError::Execute(e.to_string()))?;

        if result == 0 {
            return Err(BackendError::Execute("unexpected EOF".to_string()));
        }

        // Parse JSON response
        let response: serde_json::Value = serde_json::from_str(&line)
            .map_err(|e| BackendError::Execute(format!("invalid response: {}", e)))?;

        if let Some(error) = response.get("error").and_then(|e| e.as_str()) {
            return Ok(QueryResult::error(error));
        }

        let rows: Vec<Vec<String>> = response.get("rows")
            .and_then(|r| serde_json::from_value(r.clone()).ok())
            .unwrap_or_default();

        Ok(QueryResult::success(rows))
    }

    async fn close(mut self: Box<Self>) -> Result<(), BackendError> {
        // Send close command
        if let Some(stdin) = self.child.stdin.as_mut() {
            let _ = stdin.write_all(b"{\"type\":\"close\"}\n").await;
        }

        // Wait for process to exit
        let _ = self.child.wait().await;
        Ok(())
    }
}```
```
---
## `text_file:testing/runner/docs/architecture.md:109:5`

```markdown
```rust
pub fn compare_unordered(actual: &[Vec<String>], expected: &[String]) -> ComparisonResult```
```
---
## `text_file:testing/runner/docs/architecture.md:120:6`

```markdown
```rust
pub fn compare_error(actual_error: Option<&str>, expected_pattern: Option<&str>) -> ComparisonResult```
```
---
## `text_file:testing/runner/docs/architecture.md:131:7`

```markdown
```rust
pub enum ComparisonResult {
    /// Results match
    Match,
    /// Results don't match
    Mismatch { reason: String },
}```
```
---
## `text_file:testing/runner/docs/architecture.md:142:8`

```markdown
```rust
pub struct TestResult {
    /// Name of the test
    pub name: String,
    /// Outcome of the test
    pub outcome: TestOutcome,
    /// Duration of the test
    pub duration: Duration,
}

pub enum TestOutcome {
    Passed,
    Failed { reason: String },
    Skipped { reason: String },
    Error { message: String },
}```
```
---
## `text_file:testing/runner/docs/architecture.md:229:9`

```markdown
```rust
#[async_trait]
pub trait SqlBackend: Send + Sync {
    fn name(&self) -> &str;
    async fn create_database(&self, config: &DatabaseConfig)
        -> Result<Box<dyn DatabaseInstance>, BackendError>;
}

#[async_trait]
pub trait DatabaseInstance: Send + Sync {
    /// Execute setup SQL (may buffer for memory databases)
    async fn execute_setup(&mut self, sql: &str) -> Result<(), BackendError>;
    /// Execute SQL and return results (includes buffered setup SQL)
    async fn execute(&mut self, sql: &str) -> Result<QueryResult, BackendError>;
    async fn close(self: Box<Self>) -> Result<(), BackendError>;
}```
```
---
## `text_file:testing/runner/docs/architecture.md:39:0`

```markdown
```rust
#[async_trait]
pub trait SqlBackend: Send + Sync {
    /// Name of this backend (for filtering and display)
    fn name(&self) -> &str;

    /// Create a new isolated database instance
    async fn create_database(&self, config: &DatabaseConfig) -> Result<Box<dyn DatabaseInstance>>;
}```
```
---
## `text_file:testing/runner/docs/architecture.md:54:1`

```markdown
```rust
#[async_trait]
pub trait DatabaseInstance: Send + Sync {
    /// Execute SQL and return results
    async fn execute(&mut self, sql: &str) -> Result<QueryResult>;

    /// Close and cleanup the database
    async fn close(self: Box<Self>) -> Result<()>;
}```
```
---
## `text_file:testing/runner/docs/architecture.md:69:2`

```markdown
```rust
pub struct QueryResult {
    /// Rows returned, each row is a vector of string-formatted columns
    pub rows: Vec<Vec<String>>,
    /// Error message if the query failed
    pub error: Option<String>,
}```
```
---
## `text_file:testing/runner/docs/architecture.md:86:3`

```markdown
```rust
pub fn compare_exact(actual: &[Vec<String>], expected: &[String]) -> ComparisonResult```
```
---
## `text_file:testing/runner/docs/architecture.md:98:4`

```markdown
```rust
pub fn compare_pattern(actual: &[Vec<String>], pattern: &str) -> ComparisonResult```
```
---
## `text_file:testing/runner/docs/backends/cli.md:135:3`

```markdown
```rust
pub struct CliBackend {
    binary_path: PathBuf,
    working_dir: Option<PathBuf>,
    timeout: Duration,  // Default: 30 seconds
}

pub struct CliDatabaseInstance {
    binary_path: PathBuf,
    working_dir: Option<PathBuf>,
    db_path: String,
    readonly: bool,
    timeout: Duration,
    _temp_file: Option<NamedTempFile>,  // Keeps temp file alive
}```
```
---
## `text_file:testing/runner/docs/backends/cli.md:170:4`

```markdown
```rust
let backend = CliBackend::new("./target/debug/tursodb")
    .with_working_dir("/path/to/workdir")
    .with_timeout(Duration::from_secs(60));```
```
---
## `text_file:testing/runner/docs/backends/cli.md:178:5`

```markdown
```rust
async fn execute(&mut self, sql: &str) -> Result<QueryResult, BackendError> {
    // 1. Build command with args
    let mut cmd = Command::new(&self.binary_path);
    cmd.arg(&self.db_path).arg("-m").arg("list");

    // 2. Set up pipes
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    // 3. Spawn and write SQL
    let mut child = cmd.spawn()?;
    child.stdin.as_mut().unwrap().write_all(sql.as_bytes()).await?;
    child.stdin.take();  // Close stdin

    // 4. Wait with timeout
    let output = timeout(self.timeout, child.wait_with_output()).await??;

    // 5. Parse and return
    Ok(QueryResult::success(parse_list_output(&stdout)))
}```
```
---
## `text_file:testing/runner/docs/backends/cli.md:26:0`

```markdown
```rust
pub struct CliBackend {
    /// Path to the tursodb binary
    binary_path: PathBuf,
    /// Working directory for the CLI
    working_dir: Option<PathBuf>,
    /// Timeout for query execution
    timeout: Duration,
}```
```
---
## `text_file:testing/runner/docs/backends/cli.md:39:1`

```markdown
```rust
let backend = CliBackend::new("./target/debug/tursodb")
    .with_timeout(Duration::from_secs(30));

let config = DatabaseConfig {
    location: DatabaseLocation::Memory,
    readonly: false,
};

let mut db = backend.create_database(&config).await?;
let result = db.execute("SELECT 1;").await?;
db.close().await?;```
```
---
## `text_file:testing/runner/docs/backends/cli.md:89:2`

```markdown
```bash
# What the backend does internally:
echo "SELECT 1, 'hello';" | tursodb :memory: -m list

# Output:
1|hello```
```
---
## `text_file:testing/runner/docs/cli-usage.md:115:5`

```markdown
```json
{
  "files": [
    {
      "path": "tests/select.sqltest",
      "results": [
        {
          "name": "select-const-1",
          "outcome": "passed",
          "duration_ms": 2
        },
        {
          "name": "select-join",
          "outcome": "failed",
          "reason": "expected: 1|Alice\nactual: 1|Bob",
          "duration_ms": 3
        }
      ],
      "duration_ms": 15
    }
  ],
  "summary": {
    "total": 6,
    "passed": 4,
    "failed": 1,
    "skipped": 1,
    "errors": 0,
    "duration_ms": 127
  }
}```
```
---
## `text_file:testing/runner/docs/cli-usage.md:177:6`

```markdown
```yaml
- name: Run SQL tests
  run: |
    test-runner run tests/ \
      --binary ./target/release/tursodb \
      --output json \
      > test-results.json

- name: Upload results
  uses: actions/upload-artifact@v3
  with:
    name: test-results
    path: test-results.json```
```
---
## `text_file:testing/runner/docs/cli-usage.md:211:7`

```markdown
```rust
#[derive(Parser)]
#[command(name = "test-runner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run { /* ... */ },
    Check { /* ... */ },
}```
```
---
## `text_file:testing/runner/docs/cli-usage.md:22:1`

```markdown
```bash
test-runner run <PATHS>... [OPTIONS]```
```
---
## `text_file:testing/runner/docs/cli-usage.md:230:8`

```markdown
```rust
#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Commands::Run { .. } => run_tests(...).await,
        Commands::Check { paths } => check_files(paths),
    }
}```
```
---
## `text_file:testing/runner/docs/cli-usage.md:245:9`

```markdown
```rust
pub trait OutputFormat {
    fn write_test(&mut self, result: &TestResult);
    fn write_file(&mut self, result: &FileResult);
    fn write_summary(&mut self, summary: &RunSummary);
    fn flush(&mut self);
}

pub enum Format {
    Pretty,
    Json,
}```
```
---
## `text_file:testing/runner/docs/cli-usage.md:292:10`

```markdown
```rust
fn check_files(paths: Vec<PathBuf>) -> ExitCode {
    for path in paths {
        if path.is_dir() {
            // Glob for *.sqltest and check each
        } else {
            // Parse single file
        }
    }
    // Return success if no parse errors
}```
```
---
## `text_file:testing/runner/docs/cli-usage.md:307:11`

```markdown
```rust
if summary.is_success() {
    ExitCode::SUCCESS  // 0
} else {
    ExitCode::from(1)  // 1 for test failures
}
// 2 is returned for invalid arguments (before running)```
```
---
## `text_file:testing/runner/docs/cli-usage.md:42:2`

```markdown
```bash
# Run all tests in a directory
test-runner run tests/

# Run a specific test file
test-runner run tests/select.sqltest

# Run multiple paths
test-runner run tests/select.sqltest tests/insert/

# Custom binary path
test-runner run tests/ --binary ./target/release/tursodb

# Filter by test name
test-runner run tests/ -f "select-*"
test-runner run tests/ -f "*-join-*"

# Limit concurrent jobs
test-runner run tests/ -j 4

# JSON output for CI
test-runner run tests/ -o json```
```
---
## `text_file:testing/runner/docs/cli-usage.md:70:3`

```markdown
```bash
test-runner check <PATHS>...```
```
---
## `text_file:testing/runner/docs/cli-usage.md:76:4`

```markdown
```bash
# Check a single file
test-runner check tests/select.sqltest

# Check all files in a directory
test-runner check tests/```
```
---
## `text_file:testing/runner/docs/cli-usage.md:8:0`

```markdown
```bash
# Build from source
cargo build --release

# The binary will be at:
./target/release/test-runner```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:133:1`

```markdown
```sql
setup users {
    CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER);
    INSERT INTO users VALUES (1, 'Alice', 30), (2, 'Bob', 25);
}

setup products {
    CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL);
    INSERT INTO products VALUES (1, 'Widget', 9.99);
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:167:2`

```markdown
```sql
@database :memory:
@skip-file-if mvcc "MVCC not supported for this file"
@requires-file trigger "all tests need trigger support"

test example {
    SELECT 1;
}
expect {
    1
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:253:3`

```markdown
```sql
# Test with no setup
test select-constant {
    SELECT 42;
}
expect {
    42
}

# Test with single setup
@setup users
test select-users {
    SELECT id, name FROM users ORDER BY id;
}
expect {
    1|Alice
    2|Bob
}

# Test composing multiple setups
@setup users
@setup products
test select-join {
    SELECT u.name, p.name, p.price
    FROM users u, products p
    WHERE u.id = 1 LIMIT 1;
}
expect {
    Alice|Widget|9.99
}

# Test expecting error
test select-missing-table {
    SELECT * FROM nonexistent;
}
expect error {
    no such table
}

# Test with regex pattern
test select-random {
    SELECT random();
}
expect pattern {
    ^-?\d+$
}

# Test with unordered comparison
@setup users
test select-unordered {
    SELECT name FROM users;
}
expect unordered {
    Bob
    Alice
}

# Skipped test (unconditional)
@skip "known bug #123"
test select-buggy-feature {
    SELECT buggy();
}
expect {
    result
}

# Conditionally skipped test (only skipped in MVCC mode)
@skip-if mvcc "total_changes not supported in MVCC mode"
test total-changes {
    CREATE TABLE t (id INTEGER PRIMARY KEY);
    INSERT INTO t VALUES (1), (2), (3);
    SELECT total_changes();
}
expect {
    3
}

# Backend-specific test (only runs with CLI backend)
@backend cli
test cli-specific-feature {
    SELECT sqlite_version();
}
expect pattern {
    ^3\.\d+\.\d+$
}

# Backend-specific test (only runs with Rust backend)
@backend rust
test rust-specific-feature {
    SELECT 'rust-only';
}
expect {
    rust-only
}

# Test requiring a specific capability
@requires trigger "this test uses triggers"
test trigger-test {
    CREATE TABLE t (id INTEGER PRIMARY KEY);
    CREATE TRIGGER tr AFTER INSERT ON t BEGIN SELECT 1; END;
    INSERT INTO t VALUES (1);
    SELECT 'triggered';
}
expect {
    triggered
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:404:4`

```markdown
```sql
@database :memory:

setup schema {
    CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
    CREATE INDEX idx_users_name ON users(name);
}

setup data {
    INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob');
}

# Snapshot captures EXPLAIN QUERY PLAN + EXPLAIN output
@setup schema
@setup data
snapshot query-plan-by-id {
    SELECT * FROM users WHERE id = 1;
}

@setup schema
@setup data
snapshot query-plan-by-name {
    SELECT * FROM users WHERE name = 'Alice';
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:513:6`

```markdown
```sql
@setup users      # Executed first
@setup products   # Executed second
@setup orders     # Executed third
test my-test {
    ...
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:534:7`

```markdown
```sql
# This is a comment
@database :memory:

# Another comment
test example {
    SELECT 1;
}
expect {
    1
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:580:8`

```markdown
```rust
// Core types in src/parser/ast.rs
pub struct TestFile {
    pub databases: Vec<DatabaseConfig>,
    pub setups: HashMap<String, String>,
    pub tests: Vec<TestCase>,
    pub snapshots: Vec<SnapshotCase>,
    pub global_skip: Option<Skip>,        // @skip-file / @skip-file-if
    pub global_requires: Vec<Requirement>, // @requires-file
}

pub struct TestCase {
    pub name: String,
    pub sql: String,
    pub expectations: Expectations,
    pub modifiers: CaseModifiers,
}

pub struct SnapshotCase {
    pub name: String,
    pub sql: String,
    pub modifiers: CaseModifiers,
}

pub struct CaseModifiers {
    pub setups: Vec<SetupRef>,
    pub skip: Option<Skip>,
    pub backend: Option<Backend>,
    pub requires: Vec<Requirement>,
}

pub struct Skip {
    pub reason: String,
    pub condition: Option<SkipCondition>,
}

pub enum SkipCondition {
    Mvcc,  // Skip when MVCC mode is enabled
    Sqlite, // Skip when running against sqlite CLI backend
}

pub enum Capability {
    Trigger,           // CREATE TRIGGER support
    Strict,            // STRICT tables support
    MaterializedViews, // Materialized views (experimental)
}

pub struct Requirement {
    pub capability: Capability,
    pub reason: String,
}

pub enum Backend {
    Rust,
    Cli,
    Js,
}

pub enum Expectation {
    Exact(Vec<String>),
    Pattern(String),
    Unordered(Vec<String>),
    Error(Option<String>),
}```
```
---
## `text_file:testing/runner/docs/dsl-spec.md:96:0`

```markdown
```sql
# Writable file - both memory and temp allowed
@database :memory:
@database :temp:

# Readonly file - only readonly paths allowed
@database testing/testing.db readonly
@database testing/testing_small.db readonly

# Default databases - pre-generated with fake data
@database :default:

# Default database without rowid alias
@database :default-no-rowidalias:```
```
---
## `text_file:testing/runner/docs/parallelism.md:110:0`

```markdown
```bash
test-runner run tests/ -j 8  # Max 8 concurrent tests```
```
---
## `text_file:testing/runner/docs/parallelism.md:120:1`

```markdown
```rust
let semaphore = Arc::new(Semaphore::new(max_jobs));

for test in tests {
    let permit = semaphore.clone().acquire_owned().await?;
    tokio::spawn(async move {
        let result = run_test(test).await;
        drop(permit);  // Release permit
        result
    });
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:163:2`

```markdown
```rust
let mut futures = FuturesUnordered::new();

for task in tasks {
    futures.push(task);
}

while let Some(result) = futures.next().await {
    results.push(result?);
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:200:3`

```markdown
```rust
/// Test runner configuration
pub struct RunnerConfig {
    pub max_jobs: usize,           // Default: num_cpus::get()
    pub filter: Option<String>,    // Glob pattern for test names
}

/// Main test runner - generic over backend
pub struct TestRunner<B: SqlBackend> {
    backend: Arc<B>,
    config: RunnerConfig,
    semaphore: Arc<Semaphore>,  // Shared across all tasks
}

/// Result types
pub struct TestResult {
    pub name: String,
    pub file: PathBuf,
    pub database: DatabaseConfig,
    pub outcome: TestOutcome,
    pub duration: Duration,
}

pub enum TestOutcome {
    Passed,
    Failed { reason: String },
    Skipped { reason: String },
    Error { message: String },
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:235:4`

```markdown
```rust
pub async fn run_paths(&self, paths: &[PathBuf]) -> Result<Vec<FileResult>, BackendError> {
    // 1. Discover and parse all files (sync)
    let mut test_files: Vec<(PathBuf, TestFile)> = Vec::new();
    // ... collect files ...

    // 2. Spawn ALL tasks from ALL files at once
    let mut all_futures: FuturesUnordered<_> = FuturesUnordered::new();

    for (path, test_file) in &test_files {
        let file_futures = self.spawn_file_tests(path, test_file);
        for future in file_futures {
            let path = path.clone();
            all_futures.push(async move { (path, future.await) });
        }
    }

    // 3. Collect results as they complete
    let mut results_by_file: HashMap<PathBuf, Vec<TestResult>> = HashMap::new();

    while let Some((path, result)) = all_futures.next().await {
        results_by_file.entry(path).or_default().push(result?);
    }

    // 4. Convert to FileResults
    // ...
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:268:5`

```markdown
```rust
fn spawn_file_tests(&self, path: &Path, test_file: &TestFile)
    -> FuturesUnordered<JoinHandle<TestResult>>
{
    let futures = FuturesUnordered::new();

    for db_config in &test_file.databases {
        for test in &test_file.tests {
            // Apply filter
            if !matches_filter(&test.name, &self.config.filter) {
                continue;
            }

            let semaphore = Arc::clone(&self.semaphore);
            // Clone other data...

            futures.push(tokio::spawn(async move {
                let _permit = semaphore.acquire_owned().await.unwrap();
                run_single_test(backend, file_path, db_config, test, setups).await
            }));
        }
    }

    futures
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:297:6`

```markdown
```rust
async fn run_single_test<B: SqlBackend>(
    backend: Arc<B>,
    file_path: PathBuf,
    db_config: DatabaseConfig,
    test: TestCase,
    setups: HashMap<String, String>,
) -> TestResult {
    let start = Instant::now();

    // 1. Check if skipped
    if let Some(reason) = &test.skip {
        return TestResult { outcome: Skipped { reason }, ... };
    }

    // 2. Create database
    let mut db = backend.create_database(&db_config).await?;

    // 3. Run setups in order
    for setup_name in &test.setups {
        db.execute(setups.get(setup_name)?).await?;
    }

    // 4. Execute test SQL
    let result = db.execute(&test.sql).await?;

    // 5. Close and compare
    db.close().await;
    let comparison = compare(&result, &test.expectation);

    TestResult {
        outcome: match comparison {
            Match => Passed,
            Mismatch { reason } => Failed { reason },
        },
        duration: start.elapsed(),
        ...
    }
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:342:7`

```markdown
```rust
fn matches_filter(name: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            name.starts_with(parts[0]) && name.ends_with(parts[1])
        } else {
            parts.iter().all(|p| p.is_empty() || name.contains(p))
        }
    } else {
        name == pattern
    }
}```
```
---
## `text_file:testing/runner/docs/parallelism.md:359:8`

```markdown
```rust
pub fn summarize(results: &[FileResult]) -> RunSummary {
    let mut summary = RunSummary::default();

    for file_result in results {
        for test_result in &file_result.results {
            summary.add(&test_result.outcome);
        }
    }

    summary
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:109:4`

```markdown
```yaml
---
source: my-test.sqltest
expression: SELECT * FROM users WHERE id = 1;
info:
  statement_type: SELECT
  tables:
  - users
  setup_blocks:
  - schema
  database: ':memory:'
---
QUERY PLAN
`--SEARCH users USING INTEGER PRIMARY KEY (rowid=?)

BYTECODE
addr  opcode       p1  p2  p3  p4          p5  comment
   0  Init          0   8   0               0  Start at 8
   1  OpenRead      0   2   0  k(3,B,B,B)   0  table=users, root=2, iDb=0
   ...```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:162:5`

```markdown
```bash
# Default mode (auto)
cargo run --bin test-runner -- run tests/

# Accept all snapshot changes
cargo run --bin test-runner -- run tests/ --snapshot-mode=always

# Review mode (create .snap.new files)
cargo run --bin test-runner -- run tests/ --snapshot-mode=new

# Read-only mode (CI)
cargo run --bin test-runner -- run tests/ --snapshot-mode=no

# Filter specific snapshots
cargo run --bin test-runner -- run tests/ --snapshot-filter="query-plan*"```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:181:6`

```markdown
```bash
cargo run --bin test-runner -- check tests/```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:192:7`

```markdown
```bash
# Run all tests including snapshots
make -C test-runner run-cli

# Run examples (includes snapshot examples)
make -C test-runner run-examples

# Check syntax and pending snapshots
make -C test-runner check```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:207:8`

```markdown
```yaml
# .github/workflows/test.yml
- name: Run SQL tests
  run: |
    cargo run --bin test-runner -- run tests/ --snapshot-mode=no

- name: Check for pending snapshots
  run: |
    cargo run --bin test-runner -- check tests/```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:22:0`

```markdown
```sql
@database :memory:

setup schema {
    CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
    CREATE INDEX idx_users_name ON users(name);
}

@setup schema
snapshot my-query-plan {
    SELECT * FROM users WHERE id = 1;
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:233:9`

```markdown
```sql
@database :memory:

snapshot query-plan {
    SELECT * FROM users;
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:243:10`

```markdown
```sql
@database :memory:

setup schema {
    CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);
}

setup data {
    INSERT INTO users VALUES (1, 'Alice');
}

@setup schema
@setup data
snapshot query-plan-with-data {
    SELECT * FROM users WHERE id = 1;
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:263:11`

```markdown
```sql
# Unconditional skip
@skip "query plan not stable yet"
snapshot unstable-plan {
    SELECT * FROM complex_view;
}

# Conditional skip (MVCC mode)
@skip-if mvcc "different plan in MVCC mode"
snapshot standard-plan {
    SELECT * FROM users;
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:281:12`

```markdown
```sql
# Explicitly require Rust backend (optional, since it's the only backend that runs snapshots)
@backend rust
snapshot turso-query-plan {
    SELECT * FROM users WHERE id = 1;
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:291:13`

```markdown
```sql
# Snapshot requires trigger support
@requires trigger "query plan involves triggers"
@setup schema-with-triggers
snapshot trigger-query-plan {
    INSERT INTO audit_log SELECT * FROM events;
}```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:38:1`

```markdown
```bash
# First run creates .snap.new files for review
make -C test-runner run-cli

# Or directly:
cargo run --bin test-runner -- run tests/my-test.sqltest```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:48:2`

```markdown
```bash
# Review and accept all pending snapshots
cargo run --bin test-runner -- run tests/ --snapshot-mode=always```
```
---
## `text_file:testing/runner/docs/snapshot-testing.md:55:3`

```markdown
```bash
git add tests/snapshots/
git commit -m "Add query plan snapshots"```
```
