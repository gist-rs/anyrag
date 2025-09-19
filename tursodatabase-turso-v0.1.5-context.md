# Code Examples for tursodatabase-turso (Version: v0.1.5)

## `doc_comment:bindings/java/rs_src/utils.rs:17:0`
**Source:** `bindings/java/rs_src/utils.rs` (`doc_comment`)

```rust
set_err_msg_and_throw_exception(env, obj, Codes::SQLITE_ERROR, "An error occurred".to_string());
```
---
## `doc_comment:bindings/rust/src/params.rs:103:0`
**Source:** `bindings/rust/src/params.rs` (`doc_comment`)

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
## `doc_comment:vendored/sqlite3-parser/src/lexer/sql/mod.rs:303:0`
**Source:** `vendored/sqlite3-parser/src/lexer/sql/mod.rs` (`doc_comment`)

```rust
use turso_sqlite3_parser::lexer::sql::Tokenizer;
use turso_sqlite3_parser::lexer::Scanner;

let tokenizer = Tokenizer::new();
let input = b"PRAGMA parser_trace=ON;";
let mut s = Scanner::new(tokenizer);
let Ok((_, Some((token1, _)), _)) = s.scan(input) else { panic!() };
s.scan(input).unwrap();
assert!(b"PRAGMA".eq_ignore_ascii_case(token1));
```
---
## `example_file:bindings/rust/examples/example.rs`
**Source:** `bindings/rust/examples/example.rs` (`example_file`)

```rust
use turso::Builder;

#[tokio::main]
async fn main() {
    let db = Builder::new_local(":memory:").build().await.unwrap();

    let conn = db.connect().unwrap();

    conn.query("select 1; select 1;", ()).await.unwrap();

    conn.execute("CREATE TABLE IF NOT EXISTS users (email TEXT)", ())
        .await
        .unwrap();

    conn.pragma_query("journal_mode", |row| {
        println!("{:?}", row.get_value(0));
        Ok(())
    })
    .unwrap();

    let mut stmt = conn
        .prepare("INSERT INTO users (email) VALUES (?1)")
        .await
        .unwrap();

    stmt.execute(["foo@example.com"]).await.unwrap();

    let mut stmt = conn
        .prepare("SELECT * FROM users WHERE email = ?1")
        .await
        .unwrap();

    let mut rows = stmt.query(["foo@example.com"]).await.unwrap();

    let row = rows.next().await.unwrap().unwrap();

    let value = row.get_value(0).unwrap();

    println!("Row: {value:?}");
}

```
---
## `example_file:vendored/sqlite3-parser/examples/sql_check.rs`
**Source:** `vendored/sqlite3-parser/examples/sql_check.rs` (`example_file`)

```rust
use fallible_iterator::FallibleIterator;
use std::env;
use std::fs::read;
use std::panic;

use turso_sqlite3_parser::lexer::sql::Parser;

/// Parse specified files and check all commands.
fn main() {
    env_logger::init();
    let args = env::args();
    for arg in args.skip(1) {
        println!("{arg}");
        let result = panic::catch_unwind(|| {
            let input = read(arg.clone()).unwrap();
            let mut parser = Parser::new(&input);
            loop {
                match parser.next() {
                    Ok(None) => break,
                    Err(err) => {
                        eprintln!("Err: {err} in {arg}");
                        break;
                    }
                    Ok(Some(cmd)) => {
                        let input = cmd.to_string();
                        let mut checker = Parser::new(input.as_bytes());
                        match checker.next() {
                            Err(err) => {
                                eprintln!(
                                    "Check Err in {}:{}, {} in\n{}\n{:?}",
                                    arg,
                                    parser.line(),
                                    err,
                                    input,
                                    cmd
                                );
                            }
                            Ok(None) => {
                                eprintln!("Check Err in {}:{}, {:?}", arg, parser.line(), cmd);
                            }
                            Ok(Some(check)) => {
                                if cmd != check {
                                    eprintln!("{cmd:?}\n<>\n{check:?}");
                                }
                            }
                        }
                    }
                }
            }
        });
        if let Err(e) = result {
            eprintln!("Panic: {e:?} in {arg}");
        }
    }
}

```
---
## `example_file:vendored/sqlite3-parser/examples/sql_cmd.rs`
**Source:** `vendored/sqlite3-parser/examples/sql_cmd.rs` (`example_file`)

```rust
use std::env;

use fallible_iterator::FallibleIterator;
use turso_sqlite3_parser::lexer::sql::Parser;

/// Parse args.
// RUST_LOG=sqlite3Parser=debug
fn main() {
    env_logger::init();
    let args = env::args();
    for arg in args.skip(1) {
        let mut parser = Parser::new(arg.as_bytes());
        loop {
            match parser.next() {
                Ok(None) => break,
                Err(err) => {
                    eprintln!("Err: {err} in {arg}");
                    break;
                }
                Ok(Some(cmd)) => {
                    println!("{cmd}");
                }
            }
        }
    }
}

```
---
## `example_file:vendored/sqlite3-parser/examples/sql_cmds.rs`
**Source:** `vendored/sqlite3-parser/examples/sql_cmds.rs` (`example_file`)

```rust
use fallible_iterator::FallibleIterator;
use std::env;
use std::fs::read;
use std::panic;

#[cfg(not(feature = "YYNOERRORRECOVERY"))]
use turso_sqlite3_parser::lexer::sql::Error;
use turso_sqlite3_parser::lexer::sql::Parser;

/// Parse specified files and print all commands.
fn main() {
    env_logger::init();
    let args = env::args();
    for arg in args.skip(1) {
        println!("{arg}");
        let result = panic::catch_unwind(|| {
            let input = read(arg.clone()).unwrap();
            let mut parser = Parser::new(input.as_ref());
            loop {
                match parser.next() {
                    Ok(None) => break,
                    Err(err) => {
                        eprintln!("Err: {err} in {arg}");
                        #[cfg(feature = "YYNOERRORRECOVERY")]
                        break;
                        #[cfg(not(feature = "YYNOERRORRECOVERY"))]
                        if let Error::ParserError(..) = err {
                        } else {
                            break;
                        }
                    }
                    Ok(Some(cmd)) => {
                        println!("{cmd}");
                    }
                }
            }
        });
        if let Err(e) = result {
            eprintln!("Panic: {e:?} in {arg}");
        }
    }
}

```
---
## `example_file:vendored/sqlite3-parser/examples/sql_tokens.rs`
**Source:** `vendored/sqlite3-parser/examples/sql_tokens.rs` (`example_file`)

```rust
use turso_sqlite3_parser::lexer::sql::{TokenType, Tokenizer};
use turso_sqlite3_parser::lexer::Scanner;

use std::env;
use std::fs::read;
use std::str;

/// Tokenize specified files (and do some checks)
fn main() {
    use TokenType::*;
    let args = env::args();
    for arg in args.skip(1) {
        let input = read(arg.clone()).unwrap();
        let tokenizer = Tokenizer::new();
        let mut s = Scanner::new(tokenizer);
        loop {
            match s.scan(&input) {
                Ok((_, None, _)) => break,
                Err(err) => {
                    //eprintln!("{} at line: {}, column: {}", err, s.line(), s.column());
                    eprintln!("Err: {err} in {arg}");
                    break;
                }
                Ok((_, Some((token, token_type)), _)) => match token_type {
                    TK_TEMP => debug_assert!(
                        b"TEMP".eq_ignore_ascii_case(token)
                            || b"TEMPORARY".eq_ignore_ascii_case(token)
                    ),
                    TK_EQ => debug_assert!(b"=" == token || b"==" == token),
                    TK_NE => debug_assert!(b"<>" == token || b"!=" == token),
                    //TK_STRING => debug_assert!(),
                    //TK_ID => debug_assert!(),
                    //TK_VARIABLE => debug_assert!(),
                    TK_BLOB => debug_assert!(
                        token.len() % 2 == 0 && token.iter().all(u8::is_ascii_hexdigit)
                    ),
                    TK_INTEGER => {
                        if token.len() > 2
                            && token[0] == b'0'
                            && (token[1] == b'x' || token[1] == b'X')
                        {
                            if let Err(err) =
                                i64::from_str_radix(str::from_utf8(&token[2..]).unwrap(), 16)
                            {
                                eprintln!("Err: {err} in {arg}");
                            }
                        } else {
                            /*let raw = str::from_utf8(token).unwrap();
                            let res = raw.parse::<i64>();
                            if res.is_err() {
                                eprintln!("Err: {} in {}", res.unwrap_err(), arg);
                            }*/
                            debug_assert!(token.iter().all(u8::is_ascii_digit))
                        }
                    }
                    TK_FLOAT => {
                        debug_assert!(str::from_utf8(token).unwrap().parse::<f64>().is_ok())
                    }
                    TK_CTIME_KW => debug_assert!(
                        b"CURRENT_DATE".eq_ignore_ascii_case(token)
                            || b"CURRENT_TIME".eq_ignore_ascii_case(token)
                            || b"CURRENT_TIMESTAMP".eq_ignore_ascii_case(token)
                    ),
                    TK_JOIN_KW => debug_assert!(
                        b"CROSS".eq_ignore_ascii_case(token)
                            || b"FULL".eq_ignore_ascii_case(token)
                            || b"INNER".eq_ignore_ascii_case(token)
                            || b"LEFT".eq_ignore_ascii_case(token)
                            || b"NATURAL".eq_ignore_ascii_case(token)
                            || b"OUTER".eq_ignore_ascii_case(token)
                            || b"RIGHT".eq_ignore_ascii_case(token)
                    ),
                    TK_LIKE_KW => debug_assert!(
                        b"GLOB".eq_ignore_ascii_case(token)
                            || b"LIKE".eq_ignore_ascii_case(token)
                            || b"REGEXP".eq_ignore_ascii_case(token)
                    ),
                    _ => match token_type.as_str() {
                        Some(str) => {
                            debug_assert!(str.eq_ignore_ascii_case(str::from_utf8(token).unwrap()))
                        }
                        _ => {
                            println!("'{}', {:?}", str::from_utf8(token).unwrap(), token_type);
                        }
                    },
                },
            }
        }
    }
}

```
---
## `readme:README.md:123:0`
**Source:** `README.md` (`readme`)

```rust
let db = Builder::new_local("sqlite.db").build().await?;
let conn = db.connect()?;

let res = conn.query("SELECT * FROM users", ()).await?;
```
---
## `readme:bindings/rust/README.md:114:2`
**Source:** `bindings/rust/README.md` (`readme`)

```rust
let db = Builder::new_local(":memory:").build().await?;
let db = Builder::new_local("data.db").build().await?;
```
---
## `readme:bindings/rust/README.md:123:3`
**Source:** `bindings/rust/README.md` (`readme`)

```rust
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
## `readme:bindings/rust/README.md:140:4`
**Source:** `bindings/rust/README.md` (`readme`)

```rust
use futures_util::TryStreamExt;

let mut rows = conn.query("SELECT name, email FROM users", ()).await?;

while let Some(row) = rows.try_next().await? {
    let name = row.get_value(0)?.as_text().unwrap_or(&"".to_string());
    let email = row.get_value(1)?.as_text().unwrap_or(&"".to_string());
    println!("{}: {}", name, email);
}
```
---
## `readme:bindings/rust/README.md:30:0`
**Source:** `bindings/rust/README.md` (`readme`)

```rust
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
    
    while let Some(row) = rows.try_next().await? {
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
## `readme:bindings/rust/README.md:76:1`
**Source:** `bindings/rust/README.md` (`readme`)

```rust
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
**Source:** `extensions/core/README.md` (`readme`)

```rust
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
**Source:** `extensions/core/README.md` (`readme`)

```rust
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
**Source:** `extensions/core/README.md` (`readme`)

```rust
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
**Source:** `extensions/core/README.md` (`readme`)

```rust
use turso_ext::{ExtResult as Result, VfsDerive, VfsExtension, VfsFile};

/// Your struct must also impl Default
#[derive(VfsDerive, Default)]
struct ExampleFS;


struct ExampleFile {
    file: std::fs::File,
}

impl VfsExtension for ExampleFS {
    /// The name of your vfs module
    const NAME: &'static str = "example";

    type File = ExampleFile;

    fn open(&self, path: &str, flags: i32, _direct: bool) -> Result<Self::File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(flags & 1 != 0)
            .open(path)
            .map_err(|_| ResultCode::Error)?;
        Ok(TestFile { file })
    }

    fn run_once(&self) -> Result<()> {
    // (optional) method to cycle/advance IO, if your extension is asynchronous
        Ok(())
    }

    fn close(&self, file: Self::File) -> Result<()> {
    // (optional) method to close or drop the file
        Ok(())
    }

    fn generate_random_number(&self) -> i64 {
    // (optional) method to generate random number. Used for testing
        let mut buf = [0u8; 8];
        getrandom::fill(&mut buf).unwrap();
        i64::from_ne_bytes(buf)
    }

   fn get_current_time(&self) -> String {
    // (optional) method to generate random number. Used for testing
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }
}

impl VfsFile for ExampleFile {
    fn read(
        &mut self,
        buf: &mut [u8],
        count: usize,
        offset: i64,
    ) -> Result<i32> {
        if file.file.seek(SeekFrom::Start(offset as u64)).is_err() {
            return Err(ResultCode::Error);
        }
        file.file
            .read(&mut buf[..count])
            .map_err(|_| ResultCode::Error)
            .map(|n| n as i32)
    }

    fn write(&mut self, buf: &[u8], count: usize, offset: i64) -> Result<i32> {
        if self.file.seek(SeekFrom::Start(offset as u64)).is_err() {
            return Err(ResultCode::Error);
        }
        self.file
            .write(&buf[..count])
            .map_err(|_| ResultCode::Error)
            .map(|n| n as i32)
    }

    fn sync(&self) -> Result<()> {
        self.file.sync_all().map_err(|_| ResultCode::Error)
    }

    fn lock(&self, _exclusive: bool) -> Result<()> {
        // (optional) method to lock the file
        Ok(())
    }

    fn unlock(&self) -> Result<()> {
       // (optional) method to lock the file
        Ok(())
    }

    fn size(&self) -> i64 {
        self.file.metadata().map(|m| m.len() as i64).unwrap_or(-1)
    }
}
```
---
## `readme:extensions/core/README.md:81:0`
**Source:** `extensions/core/README.md` (`readme`)

```rust
register_extension!{
    scalars: { double }, // name of your function, if different from attribute name
    aggregates: { Percentile },
    vtabs: { CsvVTable },
    vfs: { ExampleFS },
}
```
---
## `readme:extensions/core/README.md:95:1`
**Source:** `extensions/core/README.md` (`readme`)

```rust
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
## `readme:simulator/README.md:74:0`
**Source:** `simulator/README.md` (`readme`)

```rust
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
## `test:sqlite3/tests/compat/mod.rs:test_close`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

```rust
unsafe {
            assert_eq!(sqlite3_close(ptr::null_mut()), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_disable_wal_checkpoint`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
## `test:sqlite3/tests/compat/mod.rs:test_get_autocommit`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

```rust
unsafe {
            let version = sqlite3_libversion();
            assert!(!version.is_null());
```
---
## `test:sqlite3/tests/compat/mod.rs:test_libversion_number`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

```rust
unsafe {
            let version_num = sqlite3_libversion_number();
            assert!(version_num >= 3042000);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_open_existing`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

```rust
unsafe {
            let mut db = ptr::null_mut();
            assert_eq!(
                sqlite3_open(c"../testing/testing_clone.db".as_ptr(), &mut db),
                SQLITE_OK
            );
            assert_eq!(sqlite3_close(db), SQLITE_OK);
```
---
## `test:sqlite3/tests/compat/mod.rs:test_open_not_found`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_clear_bindings`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
            let filename_str = std::ffi::CStr::from_ptr(filename).to_str().unwrap();
            assert_eq!(filename_str, temp_file.path().to_str().unwrap());

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
## `test:sqlite3/tests/compat/mod.rs:test_sqlite3_last_insert_rowid`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
## `test:sqlite3/tests/compat/mod.rs:test_wal_frame_count`
**Source:** `sqlite3/tests/compat/mod.rs` (`test`)

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
## `test:tests/integration/functions/test_cdc.rs:test_cdc_bin_record`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn = db.connect_limbo();
    let record = record([
        Value::Null,
        Value::Integer(1),
        // use golden ratio instead of pi because clippy has weird rule that I can't use PI approximation written by hand
        Value::Real(1.61803),
        Value::Text("hello".to_string()),
    ]);
    let mut record_hex = String::new();
    for byte in record {
        record_hex.push_str(&format!("{byte:02X
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_crud`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('id')")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (20, 20), (10, 10), (5, 1)")
        .unwrap();
    conn.execute("UPDATE t SET y = 100 WHERE x = 5").unwrap();
    conn.execute("DELETE FROM t WHERE x > 5").unwrap();
    conn.execute("INSERT INTO t VALUES (1, 1)").unwrap();
    conn.execute("UPDATE t SET x = 2 WHERE x = 1").unwrap();

    let rows = limbo_exec_rows(&db, &conn, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(2), Value::Integer(1)],
            vec![Value::Integer(5), Value::Integer(100)],
        ]
    );
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(20),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(10),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(5),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(0),
                Value::Text("t".to_string()),
                Value::Integer(5),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(5),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(10),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(6),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(20),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(7),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(8),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(9),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_custom_table`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn1 = db.connect_limbo();
    conn1
        .execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc')")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    conn1.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
        ]
    );
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM custom_cdc"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_failed_op`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('id')")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 10), (2, 20)")
        .unwrap();
    assert!(conn
        .execute("INSERT INTO t VALUES (3, 30), (4, 40), (5, 10)")
        .is_err());
    conn.execute("INSERT INTO t VALUES (6, 60), (7, 70)")
        .unwrap();

    let rows = limbo_exec_rows(&db, &conn, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
            vec![Value::Integer(6), Value::Integer(60)],
            vec![Value::Integer(7), Value::Integer(70)],
        ]
    );
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(6),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(7),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_ignore_changes_in_cdc_table`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn1 = db.connect_limbo();
    conn1
        .execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc')")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    conn1.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
        ]
    );
    conn1
        .execute("DELETE FROM custom_cdc WHERE change_id < 2")
        .unwrap();
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM custom_cdc"), 1);
    assert_eq!(
        rows,
        vec![vec![
            Value::Integer(2),
            Value::Null,
            Value::Integer(1),
            Value::Text("t".to_string()),
            Value::Integer(2),
            Value::Null,
            Value::Null,
            Value::Null,
        ],]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_independent_connections`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn1 = db.connect_limbo();
    let conn2 = db.connect_limbo();
    conn1
        .execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc1')")
        .unwrap();
    conn2
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc2')")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    conn2.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)]
        ]
    );
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM custom_cdc1"), 1);
    assert_eq!(
        rows,
        vec![vec![
            Value::Integer(1),
            Value::Null,
            Value::Integer(1),
            Value::Text("t".to_string()),
            Value::Integer(1),
            Value::Null,
            Value::Null,
            Value::Null,
        ]]
    );
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM custom_cdc2"), 1);
    assert_eq!(
        rows,
        vec![vec![
            Value::Integer(1),
            Value::Null,
            Value::Integer(1),
            Value::Text("t".to_string()),
            Value::Integer(2),
            Value::Null,
            Value::Null,
            Value::Null,
        ]]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_independent_connections_different_cdc_not_ignore`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn1 = db.connect_limbo();
    let conn2 = db.connect_limbo();
    conn1
        .execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc1')")
        .unwrap();
    conn2
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc2')")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    conn1.execute("INSERT INTO t VALUES (2, 20)").unwrap();
    conn2.execute("INSERT INTO t VALUES (3, 30)").unwrap();
    conn2.execute("INSERT INTO t VALUES (4, 40)").unwrap();
    conn1
        .execute("DELETE FROM custom_cdc2 WHERE change_id < 2")
        .unwrap();
    conn2
        .execute("DELETE FROM custom_cdc1 WHERE change_id < 2")
        .unwrap();
    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
            vec![Value::Integer(3), Value::Integer(30)],
            vec![Value::Integer(4), Value::Integer(40)],
        ]
    );
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM custom_cdc1"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(-1),
                Value::Text("custom_cdc2".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ]
        ]
    );
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn2, "SELECT * FROM custom_cdc2"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(4),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(-1),
                Value::Text("custom_cdc1".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ]
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_schema_changes`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn = db.connect_limbo();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('full')")
        .unwrap();
    conn.execute("CREATE TABLE t(x, y, z UNIQUE, q, PRIMARY KEY (x, y))")
        .unwrap();
    conn.execute("CREATE TABLE q(a, b, c)").unwrap();
    conn.execute("CREATE INDEX t_q ON t(q)").unwrap();
    conn.execute("CREATE INDEX q_abc ON q(a, b, c)").unwrap();
    conn.execute("DROP TABLE t").unwrap();
    conn.execute("DROP INDEX q_abc").unwrap();
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);

    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x, y, z UNIQUE, q, PRIMARY KEY (x, y))".to_string()
                    )
                ])),
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(5),
                Value::Null,
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("q".to_string()),
                    Value::Text("q".to_string()),
                    Value::Integer(6),
                    Value::Text("CREATE TABLE q (a, b, c)".to_string())
                ])),
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(6),
                Value::Null,
                Value::Blob(record([
                    Value::Text("index".to_string()),
                    Value::Text("t_q".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(7),
                    Value::Text("CREATE INDEX t_q ON t (q)".to_string())
                ])),
                Value::Null,
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(7),
                Value::Null,
                Value::Blob(record([
                    Value::Text("index".to_string()),
                    Value::Text("q_abc".to_string()),
                    Value::Text("q".to_string()),
                    Value::Integer(8),
                    Value::Text("CREATE INDEX q_abc ON q (a, b, c)".to_string())
                ])),
                Value::Null,
            ],
            vec![
                Value::Integer(5),
                Value::Null,
                Value::Integer(-1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(2),
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x, y, z UNIQUE, q, PRIMARY KEY (x, y))".to_string()
                    )
                ])),
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(6),
                Value::Null,
                Value::Integer(-1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(7),
                Value::Blob(record([
                    Value::Text("index".to_string()),
                    Value::Text("q_abc".to_string()),
                    Value::Text("q".to_string()),
                    Value::Integer(8),
                    Value::Text("CREATE INDEX q_abc ON q (a, b, c)".to_string())
                ])),
                Value::Null,
                Value::Null,
            ]
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_schema_changes_alter_table`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn = db.connect_limbo();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('full')")
        .unwrap();
    conn.execute("CREATE TABLE t(x, y, z UNIQUE, q, PRIMARY KEY (x, y))")
        .unwrap();
    conn.execute("ALTER TABLE t DROP COLUMN q").unwrap();
    conn.execute("ALTER TABLE t ADD COLUMN t").unwrap();
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);

    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x, y, z UNIQUE, q, PRIMARY KEY (x, y))".to_string()
                    )
                ])),
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(0),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(2),
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x, y, z UNIQUE, q, PRIMARY KEY (x, y))".to_string()
                    )
                ])),
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x PRIMARY KEY, y PRIMARY KEY, z UNIQUE)".to_string()
                    )
                ])),
                Value::Blob(record([
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(1),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Text("ALTER TABLE t DROP COLUMN q".to_string())
                ])),
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(0),
                Value::Text("sqlite_schema".to_string()),
                Value::Integer(2),
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x PRIMARY KEY, y PRIMARY KEY, z UNIQUE)".to_string()
                    )
                ])),
                Value::Blob(record([
                    Value::Text("table".to_string()),
                    Value::Text("t".to_string()),
                    Value::Text("t".to_string()),
                    Value::Integer(3),
                    Value::Text(
                        "CREATE TABLE t (x PRIMARY KEY, y PRIMARY KEY, z UNIQUE, t)".to_string()
                    )
                ])),
                Value::Blob(record([
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(0),
                    Value::Integer(1),
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Text("ALTER TABLE t ADD COLUMN t".to_string())
                ])),
            ],
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_simple_after`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('after')")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 2), (3, 4)").unwrap();
    conn.execute("UPDATE t SET y = 3 WHERE x = 1").unwrap();
    conn.execute("DELETE FROM t WHERE x = 3").unwrap();
    conn.execute("DELETE FROM t WHERE x = 1").unwrap();
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);

    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Blob(record([Value::Integer(1), Value::Integer(2)])),
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Null,
                Value::Blob(record([Value::Integer(3), Value::Integer(4)])),
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(0),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Blob(record([Value::Integer(1), Value::Integer(3)])),
                Value::Null,
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(5),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ]
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_simple_before`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('before')")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 2), (3, 4)").unwrap();
    conn.execute("UPDATE t SET y = 3 WHERE x = 1").unwrap();
    conn.execute("DELETE FROM t WHERE x = 3").unwrap();
    conn.execute("DELETE FROM t WHERE x = 1").unwrap();
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);

    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(0),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Blob(record([Value::Integer(1), Value::Integer(2)])),
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Blob(record([Value::Integer(3), Value::Integer(4)])),
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(5),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Blob(record([Value::Integer(1), Value::Integer(3)])),
                Value::Null,
                Value::Null,
            ]
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_simple_full`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('full')")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 2), (3, 4)").unwrap();
    conn.execute("UPDATE t SET y = 3 WHERE x = 1").unwrap();
    conn.execute("DELETE FROM t WHERE x = 3").unwrap();
    conn.execute("DELETE FROM t WHERE x = 1").unwrap();
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);

    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Blob(record([Value::Integer(1), Value::Integer(2)])),
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Null,
                Value::Blob(record([Value::Integer(3), Value::Integer(4)])),
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(0),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Blob(record([Value::Integer(1), Value::Integer(2)])),
                Value::Blob(record([Value::Integer(1), Value::Integer(3)])),
                Value::Blob(record([
                    Value::Integer(0),
                    Value::Integer(1),
                    Value::Null,
                    Value::Integer(3)
                ])),
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Blob(record([Value::Integer(3), Value::Integer(4)])),
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(5),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Blob(record([Value::Integer(1), Value::Integer(3)])),
                Value::Null,
                Value::Null,
            ]
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_simple_id`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn.execute("PRAGMA unstable_capture_data_changes_conn('id')")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (10, 10), (5, 1)")
        .unwrap();
    let rows = limbo_exec_rows(&db, &conn, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(5), Value::Integer(1)],
            vec![Value::Integer(10), Value::Integer(10)],
        ]
    );
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn, "SELECT * FROM turso_cdc"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(10),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(5),
                Value::Null,
                Value::Null,
                Value::Null,
            ]
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_table_columns`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn = db.connect_limbo();
    conn.execute("CREATE TABLE t (a INTEGER PRIMARY KEY, b, c UNIQUE)")
        .unwrap();
    let rows = limbo_exec_rows(&db, &conn, "SELECT table_columns_json_array('t')");
    assert_eq!(
        rows,
        vec![vec![Value::Text(r#"["a","b","c"]"#.to_string())]]
    );
    conn.execute("ALTER TABLE t DROP COLUMN b").unwrap();
    let rows = limbo_exec_rows(&db, &conn, "SELECT table_columns_json_array('t')");
    assert_eq!(rows, vec![vec![Value::Text(r#"["a","c"]"#.to_string())]]);
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_transaction`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn1 = db.connect_limbo();
    conn1
        .execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1
        .execute("CREATE TABLE q (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('id,custom_cdc')")
        .unwrap();
    conn1.execute("BEGIN").unwrap();
    conn1.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    conn1.execute("INSERT INTO q VALUES (2, 20)").unwrap();
    conn1.execute("INSERT INTO t VALUES (3, 30)").unwrap();
    conn1.execute("DELETE FROM t WHERE x = 1").unwrap();
    conn1.execute("UPDATE q SET y = 200 WHERE x = 2").unwrap();
    conn1.execute("COMMIT").unwrap();
    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM t");
    assert_eq!(rows, vec![vec![Value::Integer(3), Value::Integer(30)],]);
    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM q");
    assert_eq!(rows, vec![vec![Value::Integer(2), Value::Integer(200)],]);
    let rows =
        replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM custom_cdc"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("q".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(3),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(4),
                Value::Null,
                Value::Integer(-1),
                Value::Text("t".to_string()),
                Value::Integer(1),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(5),
                Value::Null,
                Value::Integer(0),
                Value::Text("q".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ]
    );
```
---
## `test:tests/integration/functions/test_cdc.rs:test_cdc_uncaptured_connection`
**Source:** `tests/integration/functions/test_cdc.rs` (`test`)

```rust
let db = TempDatabase::new_empty(true);
    let conn1 = db.connect_limbo();
    conn1
        .execute("CREATE TABLE t (x INTEGER PRIMARY KEY, y UNIQUE)")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (1, 10)").unwrap();
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('id')")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (2, 20)").unwrap(); // captured
    let conn2 = db.connect_limbo();
    conn2.execute("INSERT INTO t VALUES (3, 30)").unwrap();
    conn2
        .execute("PRAGMA unstable_capture_data_changes_conn('id')")
        .unwrap();
    conn2.execute("INSERT INTO t VALUES (4, 40)").unwrap(); // captured
    conn2
        .execute("PRAGMA unstable_capture_data_changes_conn('off')")
        .unwrap();
    conn2.execute("INSERT INTO t VALUES (5, 50)").unwrap();

    conn1.execute("INSERT INTO t VALUES (6, 60)").unwrap(); // captured
    conn1
        .execute("PRAGMA unstable_capture_data_changes_conn('off')")
        .unwrap();
    conn1.execute("INSERT INTO t VALUES (7, 70)").unwrap();

    let rows = limbo_exec_rows(&db, &conn1, "SELECT * FROM t");
    assert_eq!(
        rows,
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(2), Value::Integer(20)],
            vec![Value::Integer(3), Value::Integer(30)],
            vec![Value::Integer(4), Value::Integer(40)],
            vec![Value::Integer(5), Value::Integer(50)],
            vec![Value::Integer(6), Value::Integer(60)],
            vec![Value::Integer(7), Value::Integer(70)],
        ]
    );
    let rows = replace_column_with_null(limbo_exec_rows(&db, &conn1, "SELECT * FROM turso_cdc"), 1);
    assert_eq!(
        rows,
        vec![
            vec![
                Value::Integer(1),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(2),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(2),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(4),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::Integer(3),
                Value::Null,
                Value::Integer(1),
                Value::Text("t".to_string()),
                Value::Integer(6),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ]
    );
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_db_share_same_file`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let mut path = TempDir::new().unwrap().keep();
    let (mut rng, _) = rng_from_time();
    path.push(format!("test-{
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_api_changed_pages`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn1
        .execute("CREATE TABLE q(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    assert_eq!(
        conn1
            .wal_changed_pages_after(0)
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([1, 2, 3])
    );
    let frames = conn1.wal_state().unwrap().max_frame;
    conn1.execute("INSERT INTO t VALUES (1, 2)").unwrap();
    conn1.execute("INSERT INTO t VALUES (3, 4)").unwrap();
    assert_eq!(
        conn1
            .wal_changed_pages_after(frames)
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([2])
    );
    let frames = conn1.wal_state().unwrap().max_frame;
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 2))")
        .unwrap();
    assert_eq!(
        conn1
            .wal_changed_pages_after(frames)
            .unwrap()
            .into_iter()
            .collect::<HashSet<_>>(),
        HashSet::from([1, 2, 4, 5])
    );
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_api_exec_commit`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();

    writer
        .execute("create table test(id integer primary key, value text)")
        .unwrap();

    writer.wal_insert_begin().unwrap();

    writer
        .execute("insert into test values (1, 'hello')")
        .unwrap();
    writer
        .execute("insert into test values (2, 'turso')")
        .unwrap();

    writer.wal_insert_end(true).unwrap();

    let mut stmt = writer.prepare("select * from test").unwrap();
    let mut rows: Vec<Vec<turso_core::types::Value>> = Vec::new();
    loop {
        let result = stmt.step();
        match result {
            Ok(StepResult::Row) => rows.push(stmt.row().unwrap().get_values().cloned().collect()),
            Ok(StepResult::IO) => db.io.run_once().unwrap(),
            Ok(StepResult::Done) => break,
            result => panic!("unexpected step result: {result:?
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_api_exec_rollback`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();

    writer
        .execute("create table test(id integer primary key, value text)")
        .unwrap();

    writer.wal_insert_begin().unwrap();

    writer
        .execute("insert into test values (1, 'hello')")
        .unwrap();
    writer
        .execute("insert into test values (2, 'turso')")
        .unwrap();

    writer.wal_insert_end(false).unwrap();

    let mut stmt = writer.prepare("select * from test").unwrap();
    let mut rows: Vec<Vec<turso_core::types::Value>> = Vec::new();
    loop {
        let result = stmt.step();
        match result {
            Ok(StepResult::Row) => rows.push(stmt.row().unwrap().get_values().cloned().collect()),
            Ok(StepResult::IO) => db.io.run_once().unwrap(),
            Ok(StepResult::Done) => break,
            result => panic!("unexpected step result: {result:?
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_api_insert_exec_mix`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();

    conn.execute("create table a(x, y)").unwrap();
    conn.execute("insert into a values (1, randomblob(1 * 4096))")
        .unwrap();
    let watermark = conn.wal_state().unwrap().max_frame;
    conn.execute("create table b(x, y)").unwrap();
    conn.execute("insert into b values (2, randomblob(2 * 4096))")
        .unwrap();

    let pages = conn.wal_changed_pages_after(watermark).unwrap();
    let mut frames = Vec::new();
    let mut frame = [0u8; 4096 + 24];
    for page_no in pages {
        let page = &mut frame[24..];
        if !conn
            .try_wal_watermark_read_page(page_no, page, Some(watermark))
            .unwrap()
        {
            continue;
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_api_revert_pages`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    let watermark1 = conn1.wal_state().unwrap().max_frame;
    conn1
        .execute("INSERT INTO t VALUES (1, randomblob(10))")
        .unwrap();
    let watermark2 = conn1.wal_state().unwrap().max_frame;

    conn1
        .execute("INSERT INTO t VALUES (3, randomblob(20))")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 2))")
        .unwrap();

    assert_eq!(
        limbo_exec_rows(&db1, &conn1, "SELECT x, length(y) FROM t"),
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(3), Value::Integer(20)],
            vec![Value::Integer(1024), Value::Integer(4096 * 2)],
        ]
    );

    revert_to(&conn1, watermark2).unwrap();

    assert_eq!(
        limbo_exec_rows(&db1, &conn1, "SELECT x, length(y) FROM t"),
        vec![vec![Value::Integer(1), Value::Integer(10)],]
    );

    revert_to(&conn1, watermark1).unwrap();

    assert_eq!(
        limbo_exec_rows(&db1, &conn1, "SELECT x, length(y) FROM t"),
        vec![] as Vec<Vec<Value>>,
    );
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_api_simulate_spilled_frames`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let (mut rng, _) = rng_from_time();
    let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn2
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    let watermark = conn1.wal_state().unwrap().max_frame;
    for _ in 0..128 {
        let key = rng.next_u32();
        let length = rng.next_u32() % 4096 + 1;
        conn1
            .execute(format!(
                "INSERT INTO t VALUES ({key
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_checkpoint_no_work`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();
    let reader = db.connect_limbo();

    writer
        .execute("create table test(id integer primary key, value text)")
        .unwrap();
    // open read txn in order to hold early WAL frames and prevent them from checkpoint
    reader.execute("BEGIN").unwrap();
    reader.execute("SELECT * FROM test").unwrap();

    writer
        .execute("insert into test values (1, 'hello')")
        .unwrap();

    writer
        .execute("insert into test values (2, 'turso')")
        .unwrap();
    writer
        .checkpoint(CheckpointMode::Passive {
            upper_bound_inclusive: None,
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_api_no_schema_changes_fuzz`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let (mut rng, _) = rng_from_time();
    for _ in 0..4 {
        let db1 = TempDatabase::new_empty(false);
        let conn1 = db1.connect_limbo();
        let db2 = TempDatabase::new_empty(false);
        let conn2 = db2.connect_limbo();
        conn1
            .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
            .unwrap();
        conn2
            .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
            .unwrap();

        let seed = rng.next_u64();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        println!("SEED: {seed
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_conflict`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn2
        .execute("CREATE TABLE q(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    assert_eq!(conn1.wal_state().unwrap().max_frame, 2);
    let mut frame = [0u8; 24 + 4096];
    conn2.wal_insert_begin().unwrap();
    conn1.wal_get_frame(1, &mut frame).unwrap();
    assert!(conn2.wal_insert_frame(1, &frame).is_err());
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_count`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();
    assert_eq!(conn.wal_state().unwrap().max_frame, 0);
    conn.execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    assert_eq!(conn.wal_state().unwrap().max_frame, 2);
    conn.execute("INSERT INTO t VALUES (10, 10), (5, 1)")
        .unwrap();
    assert_eq!(conn.wal_state().unwrap().max_frame, 3);
    conn.execute("INSERT INTO t VALUES (1024, randomblob(4096 * 10))")
        .unwrap();
    assert_eq!(conn.wal_state().unwrap().max_frame, 15);
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_far_away_write`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn2
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 10))")
        .unwrap();
    assert_eq!(conn1.wal_state().unwrap().max_frame, 14);
    let mut frame = [0u8; 24 + 4096];
    conn2.wal_insert_begin().unwrap();

    conn1.wal_get_frame(3, &mut frame).unwrap();
    conn2.wal_insert_frame(3, &frame).unwrap();

    conn1.wal_get_frame(5, &mut frame).unwrap();
    assert!(conn2.wal_insert_frame(5, &frame).is_err());
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_transfer_no_schema_changes`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn2
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (10, 10), (5, 1)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 10))")
        .unwrap();
    assert_eq!(conn1.wal_state().unwrap().max_frame, 15);
    let mut frame = [0u8; 24 + 4096];
    conn2.wal_insert_begin().unwrap();
    let frames_count = conn1.wal_state().unwrap().max_frame;
    for frame_id in 1..=frames_count {
        conn1.wal_get_frame(frame_id, &mut frame).unwrap();
        conn2.wal_insert_frame(frame_id, &frame).unwrap();
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_transfer_no_schema_changes_rollback`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn2
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 10))")
        .unwrap();
    assert_eq!(conn1.wal_state().unwrap().max_frame, 14);
    let mut frame = [0u8; 24 + 4096];
    conn2.wal_insert_begin().unwrap();
    // Intentionally leave out the final commit frame, so the big randomblob is not committed and should not be visible to transactions.
    for frame_id in 1..=(conn1.wal_state().unwrap().max_frame - 1) {
        conn1.wal_get_frame(frame_id, &mut frame).unwrap();
        conn2.wal_insert_frame(frame_id, &frame).unwrap();
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_transfer_schema_changes`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (10, 10), (5, 1)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 10))")
        .unwrap();
    assert_eq!(conn1.wal_state().unwrap().max_frame, 15);
    let mut frame = [0u8; 24 + 4096];
    let mut commits = 0;
    conn2.wal_insert_begin().unwrap();
    for frame_id in 1..=conn1.wal_state().unwrap().max_frame {
        conn1.wal_get_frame(frame_id, &mut frame).unwrap();
        let info = conn2.wal_insert_frame(frame_id, &frame).unwrap();
        if info.is_commit_frame() {
            commits += 1;
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_transfer_schema_changes_rollback`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1024, randomblob(4096 * 10))")
        .unwrap();
    assert_eq!(conn1.wal_state().unwrap().max_frame, 14);
    let mut frame = [0u8; 24 + 4096];
    conn2.wal_insert_begin().unwrap();
    for frame_id in 1..=(conn1.wal_state().unwrap().max_frame - 1) {
        conn1.wal_get_frame(frame_id, &mut frame).unwrap();
        conn2.wal_insert_frame(frame_id, &frame).unwrap();
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_frame_transfer_various_schema_changes`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db1 = TempDatabase::new_empty(false);
    let conn1 = db1.connect_limbo();
    let db2 = TempDatabase::new_empty(false);
    let conn2 = db2.connect_limbo();
    let conn3 = db2.connect_limbo();
    conn1
        .execute("CREATE TABLE t(x INTEGER PRIMARY KEY, y)")
        .unwrap();
    let mut frame = [0u8; 24 + 4096];
    let mut synced_frame = 0;
    let mut sync = || {
        let last_frame = conn1.wal_state().unwrap().max_frame;
        conn2.wal_insert_begin().unwrap();
        for frame_id in (synced_frame + 1)..=last_frame {
            conn1.wal_get_frame(frame_id, &mut frame).unwrap();
            conn2.wal_insert_frame(frame_id, &frame).unwrap();
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_revert_change_db_size`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();

    writer.execute("create table t(x, y)").unwrap();
    let watermark = writer.wal_state().unwrap().max_frame;
    writer
        .execute("insert into t values (1, randomblob(10 * 4096))")
        .unwrap();
    writer
        .execute("insert into t values (2, randomblob(20 * 4096))")
        .unwrap();
    let mut changed = writer.wal_changed_pages_after(watermark).unwrap();
    changed.sort();

    let mut frame = [0u8; 4096 + 24];

    writer.wal_insert_begin().unwrap();
    let mut frames_count = writer.wal_state().unwrap().max_frame;
    for page_no in changed {
        let page = &mut frame[24..];
        if !writer
            .try_wal_watermark_read_page(page_no, page, Some(watermark))
            .unwrap()
        {
            continue;
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_state_checkpoint_seq`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();

    writer
        .execute("create table test(id integer primary key, value text)")
        .unwrap();
    writer
        .execute("insert into test values (1, 'hello')")
        .unwrap();
    writer
        .checkpoint(CheckpointMode::Truncate {
            upper_bound_inclusive: None,
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_upper_bound_passive`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();

    writer
        .execute("create table test(id integer primary key, value text)")
        .unwrap();
    let watermark0 = writer.wal_state().unwrap().max_frame;
    writer
        .execute("insert into test values (1, 'hello')")
        .unwrap();
    let watermark1 = writer.wal_state().unwrap().max_frame;
    writer
        .execute("insert into test values (2, 'turso')")
        .unwrap();
    let watermark2 = writer.wal_state().unwrap().max_frame;
    let expected = [
        vec![
            turso_core::types::Value::Integer(1),
            turso_core::types::Value::Text(turso_core::types::Text::new("hello")),
        ],
        vec![
            turso_core::types::Value::Integer(2),
            turso_core::types::Value::Text(turso_core::types::Text::new("turso")),
        ],
    ];

    for (prefix, watermark) in [(0, watermark0), (1, watermark1), (2, watermark2)] {
        let mode = CheckpointMode::Passive {
            upper_bound_inclusive: Some(watermark),
```
---
## `test:tests/integration/functions/test_wal_api.rs:test_wal_upper_bound_truncate`
**Source:** `tests/integration/functions/test_wal_api.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let writer = db.connect_limbo();

    writer
        .execute("create table test(id integer primary key, value text)")
        .unwrap();
    writer
        .execute("insert into test values (1, 'hello')")
        .unwrap();
    let watermark = writer.wal_state().unwrap().max_frame;
    writer
        .execute("insert into test values (2, 'turso')")
        .unwrap();

    let mode = CheckpointMode::Truncate {
        upper_bound_inclusive: Some(watermark),
```
---
## `test:tests/integration/fuzz/mod.rs:concat_ws_fuzz`
**Source:** `tests/integration/fuzz/mod.rs` (`test`)

```rust
let _ = env_logger::try_init();

        let (mut rng, seed) = rng_from_time();
        log::info!("seed: {seed
```
---
## `test:tests/integration/pragma.rs:test_pragma_module_list_generate_series`
**Source:** `tests/integration/pragma.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();

    let mut rows = conn
        .query("SELECT * FROM generate_series(1, 3);")
        .expect("generate_series module not available")
        .expect("query did not return rows");

    let mut values = vec![];
    while let StepResult::Row = rows.step().unwrap() {
        let row = rows.row().unwrap();
        values.push(row.get_value(0).clone());
```
---
## `test:tests/integration/pragma.rs:test_pragma_module_list_returns_list`
**Source:** `tests/integration/pragma.rs` (`test`)

```rust
let db = TempDatabase::new_empty(false);
    let conn = db.connect_limbo();

    let mut module_list = conn.query("PRAGMA module_list;").unwrap();

    let mut counter = 0;

    if let Some(ref mut rows) = module_list {
        while let StepResult::Row = rows.step().unwrap() {
            counter += 1;
```
---
## `test:tests/integration/pragma.rs:test_pragma_page_sizes_with_writes_persists`
**Source:** `tests/integration/pragma.rs` (`test`)

```rust
for test_page_size in [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536] {
        let db = TempDatabase::new_empty(false);
        {
            {
                let conn = db.connect_limbo();
                let pragma_query = format!("PRAGMA page_size={test_page_size
```
---
## `test:tests/integration/pragma.rs:test_pragma_page_sizes_without_writes_persists`
**Source:** `tests/integration/pragma.rs` (`test`)

```rust
for test_page_size in [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536] {
        let db = TempDatabase::new_empty(false);
        {
            let conn = db.connect_limbo();
            let pragma_query = format!("PRAGMA page_size={test_page_size
```
---
## `test:tests/integration/query_processing/test_btree.rs:test_btree`
**Source:** `tests/integration/query_processing/test_btree.rs` (`test`)

```rust
let _ = env_logger::try_init();
    let mut rng = ChaCha8Rng::seed_from_u64(0);
    for depth in 0..4 {
        for attempt in 0..16 {
            let db = TempDatabase::new_with_rusqlite(
                "create table test (k INTEGER PRIMARY KEY, b BLOB);",
                false,
            );
            log::info!(
                "depth: {
```
---
## `test:tests/integration/query_processing/test_multi_thread.rs:test_schema_reprepare`
**Source:** `tests/integration/query_processing/test_multi_thread.rs` (`test`)

```rust
let tmp_db = TempDatabase::new_empty(false);
    let conn1 = tmp_db.connect_limbo();
    conn1.execute("CREATE TABLE t(x, y, z)").unwrap();
    conn1
        .execute("INSERT INTO t VALUES (1, 2, 3), (10, 20, 30)")
        .unwrap();
    let conn2 = tmp_db.connect_limbo();
    let mut stmt = conn2.prepare("SELECT y, z FROM t").unwrap();
    let mut stmt2 = conn2.prepare("SELECT x, z FROM t").unwrap();
    conn1.execute("ALTER TABLE t DROP COLUMN x").unwrap();
    assert_eq!(
        stmt2.step().unwrap_err().to_string(),
        "Parse error: no such column: x"
    );

    let mut rows = Vec::new();
    loop {
        match stmt.step().unwrap() {
            turso_core::StepResult::Done => {
                break;
```
---
## `test:tests/integration/query_processing/test_multi_thread.rs:test_schema_reprepare_write`
**Source:** `tests/integration/query_processing/test_multi_thread.rs` (`test`)

```rust
maybe_setup_tracing();
    let tmp_db = TempDatabase::new_empty(false);
    let conn1 = tmp_db.connect_limbo();
    conn1.execute("CREATE TABLE t(x, y, z)").unwrap();
    let conn2 = tmp_db.connect_limbo();
    let mut stmt = conn2.prepare("INSERT INTO t(y, z) VALUES (1, 2)").unwrap();
    let mut stmt2 = conn2.prepare("INSERT INTO t(y, z) VALUES (3, 4)").unwrap();
    conn1.execute("ALTER TABLE t DROP COLUMN x").unwrap();

    tracing::info!("Executing Stmt 1");
    loop {
        match stmt.step().unwrap() {
            turso_core::StepResult::Done => {
                break;
```
