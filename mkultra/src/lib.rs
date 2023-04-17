use rusqlite::{params, Connection};
use std::error::Error;
use std::fmt;
use std::time::{SystemTime};
use serde::{Deserialize, Serialize};
use ammonia::Builder;
use maplit::{hashset, hashmap};

const SQLITE_FILE_COMMENTS: &str = "comments.db";

#[derive(Serialize, Deserialize, Debug)]
pub struct Comment {
    pub id: u64,
    pub time: u64,
    pub user_id: String,
    pub user_name: String,
    pub comment: String,
    pub chat_id: String,
    pub picture: String,
    pub msgtype: i32,
}

impl Comment {
    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_html( field: String) -> String {
        let tags = hashset!["img"];
        let tag_attributes = hashmap!["img" => hashset!["src"]];
        let tag_blacklist = hashset!["script", "style"];
        return Builder::new()
            .tags(tags)
            .tag_attributes(tag_attributes)
            .clean_content_tags(tag_blacklist)
            .clean(&field)
            .to_string();
    }

    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_csv( field: String) -> String {
        return field.replace("\"", "\"\"").replace("\n", " ");
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Chat {
    pub id: String
}

#[derive(Debug)]
struct HydraError(String);
impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}
impl Error for HydraError {}


//Make sure the database exists and is the correct schema
pub fn init_database() -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE_COMMENTS)?;

    //Remove all previous chat id's since each run is clean
    if let Err(e) = conn.execute("CREATE TABLE IF NOT EXISTS comments (created INTEGER)", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN created INTEGER", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN uid TEXT", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN comment TEXT", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN source INTEGER", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN target TEXT", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN uname TEXT", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN cid TEXT", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN picture TEXT", params![]) {
        eprintln!("{}", e);
    }
    if let Err(e) = conn.execute("ALTER TABLE comments ADD COLUMN msgtype INTEGER", params![]) {
        eprintln!("{}", e);
    }


    Ok(true)
}

//Add a comment to the database for a certain chat session
pub fn init_chat_session_with_id(chat_id: &str) -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE_COMMENTS)?;

    //Get a current timestamp
    let timestamp: u64 = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_secs() - (86400 * 90),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    };

    match check_session(chat_id) {
        Ok(_) => {
            Ok(true)
        }
        Err(_e) => {
            //Remove all previous chat id's since each run is clean
            match conn.execute("DELETE FROM comments WHERE cid != ?1", params![chat_id])
            {
                Ok(_) => {
                    println!("Old chat sessions removed.");
                }
                Err(e) => {
                    eprintln!("{}", e);
                    return Err(Box::new(HydraError(format!("Failed to remove old chat sessions: [{}].", e).into())))
                }
            }

            //Create an initial starting comment with this chat id
            match conn.execute("INSERT INTO comments (created, uid, comment, uname, cid, picture, msgtype) \
                                              VALUES (?1,      ?2,  ?3,      ?4,    ?5,  ?6,      ?7)",
                               params![
                                   timestamp,
                                   0,
                                   "Welcome!",
                                   "",
                                   chat_id,
                                   "",
                                   -1
                               ])
            {
                Ok(_) => {
                    Ok(true)
                }
                Err(e) => {
                    eprintln!("{}", e);
                    return Err(Box::new(HydraError(format!("Failed to add initial comment: [{}].", e).into())))
                }
            }
        }
    }
}


//Add a comment to the database for a certain chat session
pub fn add_comment_to_db(comment: Comment) -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE_COMMENTS)?;

    let comment_clean = Comment {
        comment: Comment::escape_for_html(comment.comment),
        user_name: Comment::escape_for_html(comment.user_name),
        picture: Comment::escape_for_html(comment.picture),
        ..comment
    };

    match conn.execute("INSERT INTO comments (created, uid, comment, uname, cid, picture, msgtype) \
                                      VALUES (?1,      ?2,  ?3,      ?4,    ?5,  ?6,      ?7     )",
                       params![
                           comment_clean.time,
                           comment_clean.user_id,
                           comment_clean.comment,
                           comment_clean.user_name,
                           comment_clean.chat_id,
                           comment_clean.picture,
                           comment_clean.msgtype,
                       ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to add comment: [{}].", comment_clean.user_id).into())))
        }
    }
}


//Check if a chat id exists in the database
pub fn check_session(chat_id: &str) -> Result<String, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE_COMMENTS)?;

    let mut stmt = conn.prepare("\
        SELECT cid \
        FROM comments \
        WHERE cid = :cid \
        LIMIT 1 \
    ")?;
    let rows = stmt.query_map(&[(":cid", chat_id)], |row| {
        Ok(Chat {
            id: row.get(0)?
        })
    }).unwrap();

    if rows.count() < 1 {
        return Err(Box::new(HydraError(format!("No chat session found for: [{}].", chat_id).into())));
    }

    Ok("true".to_string())
}


//Get all of the comments tied to a certain chat session
pub fn get_comments_by_chat_id(chat_id: &str, msgid: u64) -> Result<Vec<Comment>, Box<dyn Error>> {
    let conn = Connection::open(SQLITE_FILE_COMMENTS)?;
    let mut comms: Vec<Comment> = Vec::new();

    //If a msgid was passed in, use it as the starting point
    let mut msg_id: String = "0".to_string();
    if msgid > 0 {
        msg_id = msgid.to_string();
    }

    //Prepare and execute the query
    let mut stmt = conn.prepare("SELECT rowid, \
                                        created, \
                                        uid, \
                                        comment, \
                                        uname, \
                                        cid, \
                                        picture,\
                                        msgtype \
                                   FROM (SELECT rowid, created, uid, comment, uname, cid, picture, msgtype \
                                           FROM comments WHERE cid = :chat_id \
                                            AND rowid > :msgid \
                                         ORDER BY rowid DESC \
                                         LIMIT :limit) \
                                 ORDER BY rowid ASC \
    ")?;
    let rows = stmt.query_map(
        &[  (":chat_id", chat_id),
            (":msgid", msg_id.as_str()),
            (":limit", "25")
        ],
        |row|
    {
        Ok(Comment {
            id: row.get(0)?,
            time: row.get(1)?,
            user_id: row.get(2)?,
            comment: row.get(3)?,
            user_name: row.get(4)?,
            chat_id: row.get(5)?,
            picture: row.get(6)?,
            msgtype: row.get(7)?,
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let comment: Comment = row.unwrap();
        comms.push(comment);
    }

    Ok(comms)
}