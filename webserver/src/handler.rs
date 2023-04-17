use crate::{Context, Response};
use hyper::StatusCode;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs;
use mkultra;



//Globals ----------------------------------------------------------------------------------------------------


//Structs ----------------------------------------------------------------------------------------------------
#[derive(Debug)]
struct HydraError(String);

impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fatal error: {}", self.0)
    }
}

impl Error for HydraError {}


//Functions --------------------------------------------------------------------------------------------------
pub async fn home(ctx: Context) -> Response {
    let chat_id: &str;

    //Get query parameters
    let params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("{:#?}", params);

    //Make sure a session param was given
    match params.get("cid") {
        Some(cid) => {
            println!("Got a chat session id: {}\n", cid);
            chat_id = cid;
        }
        None => {
            println!("Invalid chat session id.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("No chat session id given.").into())
                .unwrap();
        }
    }

    //Is the session valid
    match mkultra::check_session(chat_id) {
        Ok(_) => {
            let doc = fs::read_to_string("home.html").expect("Something went wrong reading the file.");
            return hyper::Response::builder()
                .status(StatusCode::OK)
                .body(format!("{}", doc).into())
                .unwrap();
        }
        Err(_) => {
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("That's not a live session.").into())
                .unwrap();
        }
    }

}

pub async fn pewmp3(_ctx: Context) -> Response {
    let file = fs::read("pew.mp3").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "audio/mpeg")
        .body(hyper::Body::from(file))
        .unwrap();
}

pub async fn nopicture(_ctx: Context) -> Response {
    let file = fs::read("nopicture.png").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "image/png")
        .body(hyper::Body::from(file))
        .unwrap();
}

pub async fn homejs(_ctx: Context) -> Response {
    let doc = fs::read_to_string("home.js").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc).into())
        .unwrap();
}

pub async fn utilsjs(_ctx: Context) -> Response {
    let doc = fs::read_to_string("utils.js").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc).into())
        .unwrap();
}

pub async fn nostrjs(_ctx: Context) -> Response {
    let doc = fs::read_to_string("nostr.js").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", doc).into())
        .unwrap();
}

pub async fn favicon(_ctx: Context) -> Response {
    let file = fs::read("favicon.ico").expect("Something went wrong reading the file.");
    return hyper::Response::builder()
        .status(StatusCode::OK)
        .header("Content-type", "image/x-icon")
        .body(hyper::Body::from(file))
        .unwrap();
}

pub async fn context(ctx: Context) -> Response {
    let chat_id: &str;

    //Get query parameters
    let params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
        url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
    }).unwrap_or_else(HashMap::new);

    println!("{:#?}", params);

    //Make sure a session param was given
    match params.get("cid") {
        Some(cid) => {
            println!("Got a chat session id: {}\n", cid);
            chat_id = cid;
        }
        None => {
            println!("Invalid chat session id.\n");
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("No chat session id given.").into())
                .unwrap();
        }
    }

    //Is the session valid
    match mkultra::check_session(chat_id) {
        Ok(_) => {

            //Get the comments
            let comments = mkultra::get_comments_by_chat_id(chat_id, 0).unwrap();
            let json_comments = serde_json::to_string(&comments).unwrap();

            return hyper::Response::builder()
                .status(StatusCode::OK)
                .body(format!("{}", json_comments).into())
                .unwrap();
        }
        Err(_) => {
            return hyper::Response::builder()
                .status(StatusCode::from_u16(400).unwrap())
                .body(format!("That's not a live session.").into())
                .unwrap();
        }
    }
}


// pub async fn comment(ctx: Context) -> Response {
//     let user_id: String;
//     let user_name: String;
//     let comment_text: String;
//     let chat_id: String;
//     let picture: String;
//
//     //Get a current timestamp
//     let timestamp: u64 = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
//         Ok(n) => n.as_secs() - (86400 * 90),
//         Err(_) => panic!("SystemTime before UNIX EPOCH!"),
//     };
//
//     //Get query parameters
//     let params: HashMap<String, String> = ctx.req.uri().query().map(|v| {
//         url::form_urlencoded::parse(v.as_bytes()).into_owned().collect()
//     }).unwrap_or_else(HashMap::new);
//
//     println!("{:#?}", params);
//
//     //Get the user id
//     match params.get("user_id") {
//         Some(uid) => {
//             println!("Got user id: {}\n", uid);
//             user_id = uid.to_string();
//         }
//         None => {
//             println!("No user id\n");
//             return hyper::Response::builder()
//                 .status(StatusCode::from_u16(400).unwrap())
//                 .body(format!("No user_id.").into())
//                 .unwrap();
//         }
//     }
//
//     //Get the user name
//     match params.get("user_name") {
//         Some(name) => {
//             println!("Got user name: {}\n", name);
//             user_name = name.to_string();
//         }
//         None => {
//             println!("No user name\n");
//             return hyper::Response::builder()
//                 .status(StatusCode::from_u16(400).unwrap())
//                 .body(format!("No user_name.").into())
//                 .unwrap();
//         }
//     }
//
//     //Get the comment
//     match params.get("comment") {
//         Some(comm) => {
//             println!("  {}\n", comm);
//             comment_text = comm.to_string();
//         }
//         None => {
//             println!("  No comment\n");
//             return hyper::Response::builder()
//                 .status(StatusCode::from_u16(400).unwrap())
//                 .body(format!("No comment text.").into())
//                 .unwrap();
//         }
//     }
//
//     //Get the chat session id
//     match params.get("chat_id") {
//         Some(chat) => {
//             println!("  {}\n", chat);
//             chat_id = chat.to_string();
//         }
//         None => {
//             println!("  No chat id\n");
//             return hyper::Response::builder()
//                 .status(StatusCode::from_u16(400).unwrap())
//                 .body(format!("No chat id.").into())
//                 .unwrap();
//         }
//     }
//
//     //Get the user picture
//     match params.get("picture") {
//         Some(pic) => {
//             println!("  {}\n", pic);
//             picture = pic.to_string();
//         }
//         None => {
//             println!("  No picture\n");
//             picture = "".to_string();
//         }
//     }
//
//
//     //Assemble a comment
//     let comment = mkultra::Comment {
//         id: 0,
//         time: timestamp,
//         user_id: user_id,
//         user_name: user_name,
//         comment: comment_text,
//         chat_id: chat_id,
//         picture: picture,
//         msgtype: msgtype,
//     };
//
//     match add_comment_to_db(comment) {
//         Ok(_) => {
//             println!("Added comment.\n");
//             return hyper::Response::builder()
//                 .status(StatusCode::from_u16(200).unwrap())
//                 .body(format!("OK").into())
//                 .unwrap();
//         }
//         Err(e) => {
//             eprintln!("{}", e);
//             return hyper::Response::builder()
//                 .status(StatusCode::from_u16(400).unwrap())
//                 .body(format!("ERR").into())
//                 .unwrap();
//         }
//     }
//
//
//
//     //println!("Params")
// }


// pub fn add_comment_to_db(comment: mkultra::Comment) -> Result<bool, Box<dyn Error>> {
//     let conn = Connection::open(SQLITE_FILE_COMMENTS)?;
//
//     match conn.execute("INSERT INTO comments (created, uid, comment, uname, cid, picture) \
//                                       VALUES (?1,      ?2,  ?3,      ?4,    ?5,  ?6)",
//                        params![
//                            comment.time,
//                            comment.user_id,
//                            comment.comment,
//                            comment.user_name,
//                            comment.chat_id,
//                            comment.picture
//                        ]
//     ) {
//         Ok(_) => {
//             Ok(true)
//         }
//         Err(e) => {
//             eprintln!("{}", e);
//             return Err(Box::new(HydraError(format!("Failed to add comment: [{}].", comment.user_id).into())))
//         }
//     }
// }




//Get all of the comments tied to a certain chat session
// pub fn get_comments_by_chat_id(chat_id: &str) -> Result<Vec<mkultra::Comment>, Box<dyn Error>> {
//     let conn = Connection::open(SQLITE_FILE_COMMENTS)?;
//     let mut comms: Vec<mkultra::Comment> = Vec::new();
//
//     let mut stmt = conn.prepare("SELECT rowid, created, uid, comment, uname, cid, picture \
//                                  FROM comments \
//                                  WHERE cid = :chat_id \
//                                  ORDER BY rowid ASC")?;
//     let rows = stmt.query_map(&[(":chat_id", chat_id)], |row| {
//         Ok(mkultra::Comment {
//             id: row.get(0)?,
//             time: row.get(1)?,
//             user_id: row.get(2)?,
//             comment: row.get(3)?,
//             user_name: row.get(4)?,
//             chat_id: row.get(5)?,
//             picture: row.get(6)?,
//
//         })
//     }).unwrap();
//
//     for row in rows {
//         let comment: mkultra::Comment = row.unwrap();
//         comms.push(comment);
//     }
//
//     Ok(comms)
// }