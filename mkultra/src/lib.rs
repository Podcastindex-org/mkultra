use rusqlite::{params, Connection};
use std::error::Error;
use std::fmt;
use std::time::{SystemTime};
use serde::{Deserialize, Serialize};

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
}

impl Comment {
    //Removes unsafe html interpretable characters from displayable strings
    pub fn escape_for_html( field: String) -> String {
        return field.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
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
            match conn.execute("DELETE FROM comments", params![])
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
            match conn.execute("INSERT INTO comments (created, uid, comment, uname, cid, picture) \
                                              VALUES (?1,      ?2,  ?3,      ?4,    ?5,  ?6)",
                               params![
                                   timestamp,
                                   0,
                                   "Welcome!",
                                   "",
                                   chat_id,
                                   ""
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

    match conn.execute("INSERT INTO comments (created, uid, comment, uname, cid, picture) \
                                      VALUES (?1,      ?2,  ?3,      ?4,    ?5,  ?6)",
                       params![
                           comment.time,
                           comment.user_id,
                           comment.comment,
                           comment.user_name,
                           comment.chat_id,
                           comment.picture
                       ]
    ) {
        Ok(_) => {
            Ok(true)
        }
        Err(e) => {
            eprintln!("{}", e);
            return Err(Box::new(HydraError(format!("Failed to add comment: [{}].", comment.user_id).into())))
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
                                        picture \
                                   FROM (SELECT rowid, created, uid, comment, uname, cid, picture \
                                           FROM comments WHERE cid = :chat_id \
                                            AND rowid > :msgid \
                                         ORDER BY rowid DESC \
                                         LIMIT :limit) \
                                 ORDER BY rowid ASC \
    ")?;
    let rows = stmt.query_map(
        &[  (":chat_id", chat_id),
            (":msgid", msg_id.as_str()),
            (":limit", "100")
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
        })
    }).unwrap();

    //Parse the results
    for row in rows {
        let comment: Comment = row.unwrap();

        let comment_clean = Comment {
            comment: Comment::escape_for_html(comment.comment),
            user_name: Comment::escape_for_html(comment.user_name),
            picture: Comment::escape_for_html(comment.picture),
            ..comment
        };

        comms.push(comment_clean);
    }

    Ok(comms)
}