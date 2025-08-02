use core::fmt;

use crate::command::Command;

#[derive(Debug)]
pub enum Response {
    Cmd(Command),
    Rpl(ResponseNumber),
    Unknown(String),
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Response::Cmd(cmd) => {
                write!(f, "{cmd:?}")
            }
            Response::Rpl(rpl) => {
                write!(f, "{rpl:?}")
            }
            Response::Unknown(content) => {
                write!(f, "{content:?}")
            }
        }
    }
}

/// All standard IRC RPL (Reply) numerics.
/// See: RFC 1459, RFC 2812
#[derive(Debug)]
pub enum ResponseNumber {
    /// 001: Welcome to the network
    Welcome(String),
    /// 002: Your host is...
    YourHost(String),
    /// 003: Server creation date
    Created(String),
    /// 004: Server info and supported modes
    MyInfo(String),
    /// 005: ISUPPORT/feature list (may be multi-line)
    Bounce(String),

    /// 200: Link info
    TraceLink(String),
    /// 201: Connecting info
    TraceConnecting(String),
    /// 202: Handshake info
    TraceHandshake(String),
    /// 203: Unknown class
    TraceUnknown(String),
    /// 204: Operator class
    TraceOperator(String),
    /// 205: User class
    TraceUser(String),
    /// 206: Server class
    TraceServer(String),
    /// 208: Service class
    TraceService(String),
    /// 209: New type of class
    TraceNewType(String),
    /// 210: Trace log
    TraceLog(String),

    /// 211: Stats link info
    StatsLinkInfo(String),
    /// 212: Stats command usage
    StatsCommands(String),
    /// 213: Stats operator usage
    StatsCLine(String),
    /// 214: Stats NLine
    StatsNLine(String),
    /// 215: Stats ILine
    StatsILine(String),
    /// 216: Stats KLine
    StatsKLine(String),
    /// 217: Stats QLine
    StatsQLine(String),
    /// 218: Stats YLine
    StatsYLine(String),
    /// 219: End of stats
    EndOfStats(String),

    /// 221: User mode string
    UserModeIs(String),

    /// 231: Service info
    ServiceInfo(String),
    /// 232: End of service info
    EndOfService(String),

    /// 233: Stats ULine
    StatsULine(String),
    /// 234: Stats VLine
    StatsVLine(String),
    /// 235: Stats XLine
    StatsXLine(String),

    /// 241: Stats LLine
    StatsLLine(String),
    /// 242: Stats Uptime
    StatsUptime(String),
    /// 243: Stats OLine
    StatsOLine(String),
    /// 244: Stats HLine
    StatsHLine(String),
    /// 245: Stats PLine
    StatsPLine(String),
    /// 246: Stats DLine
    StatsDLine(String),
    /// 247: Stats TLine
    StatsTLine(String),
    /// 250: Highest connection count
    HighestConnCount(String),

    /// 251: Number of users and servers
    LUserClient(String),
    /// 252: Number of IRC operators
    LUserOp(String),
    /// 253: Number of unknown connections
    LUserUnknown(String),
    /// 254: Number of channels formed
    LUserChannels(String),
    /// 255: Info about your connection
    LUserMe(String),
    /// 256: Admin info
    AdminMe(String),
    /// 257: Admin location 1
    AdminLoc1(String),
    /// 258: Admin location 2
    AdminLoc2(String),
    /// 259: Admin email
    AdminEmail(String),

    /// 261: Trace log
    TraceLog2(String),

    /// 262: End of LUser
    EndOfLUser(String),

    /// 263: Try again
    TryAgain(String),

    /// 265: Local users
    LocalUsers(String),
    /// 266: Global users
    GlobalUsers(String),

    /// 300: None (reserved)
    None(String),

    /// 301: Away message
    Away(String),
    /// 302: User host
    UserHost(String),
    /// 303: ISON reply
    Ison(String),
    /// 304: Text (unused)
    Text(String),
    /// 305: You are no longer marked as away
    UnAway(String),
    /// 306: You are now marked as away
    NowAway(String),

    /// 311: WHOIS user
    WhoisUser(String),
    /// 312: WHOIS server
    WhoisServer(String),
    /// 313: WHOIS operator
    WhoisOperator(String),
    /// 314: WHOWAS user
    WhowasUser(String),
    /// 315: End of WHO
    EndOfWho(String),
    /// 316: WHOIS idle
    WhoisIdle(String),
    /// 317: WHOIS idle time
    WhoisIdleTime(String),
    /// 318: End of WHOIS
    EndOfWhois(String),
    /// 319: WHOIS channels
    WhoisChannels(String),

    /// 321: List start
    ListStart(String),
    /// 322: List
    List(String),
    /// 323: End of list
    ListEnd(String),

    /// 324: Channel mode
    ChannelModeIs(String),
    /// 325: Unique channel ID
    UniqueOpIs(String),

    /// 331: No topic set
    NoTopic(String),
    /// 332: Topic
    Topic(String),
    /// 333: Topic who/time
    TopicWhoTime(String),

    /// 341: INVITE confirmation
    Invite(String),
    /// 342: Summon answer
    SummonAnswer(String),
    /// 346: Invite list
    InviteList(String),
    /// 347: End of invite list
    EndOfInviteList(String),
    /// 348: Exception list
    ExceptionList(String),
    /// 349: End of exception list
    EndOfExceptionList(String),

    /// 351: Version
    Version(String),
    /// 352: WHO reply
    WhoReply(String),
    /// 353: NAMES reply
    NameReply(Vec<String>),
    /// 354: WHO reply extended
    WhoReplyExtended(String),
    /// 361: KILL done
    KillDone(String),
    /// 362: Closing link
    Closing(String),
    /// 363: Links
    Links(String),
    /// 364: Links
    Links2(String),
    /// 365: End of links
    EndOfLinks(String),
    /// 366: End of NAMES
    EndOfNames(String),
    /// 367: Ban list
    BanList(String),
    /// 368: End of ban list
    EndOfBanList(String),
    /// 369: End of WHOWAS
    EndOfWhowas(String),

    /// 371: Info
    Info(String),
    /// 372: MOTD line
    MOTD(String),
    /// 373: MOTD start
    MOTDStart(String),
    /// 374: End of info
    EndOfInfo(String),
    /// 375: MOTD start
    MOTDStart2(String),
    /// 376: End of MOTD
    EndOfMOTD(String),
    /// 381: You are now an IRC operator
    YouAreOper(String),
    /// 382: Rehashing
    Rehashing(String),
    /// 383: You are service
    YouAreService(String),
    /// 391: Time
    Time(String),
    /// 392: Users start
    UsersStart(String),
    /// 393: Users
    Users(String),
    /// 394: End of users
    EndOfUsers(String),
    /// 395: No users
    NoUsers(String),

    Err(u16, String),

    /// Any other reply not explicitly listed
    Unknown(u16, String),
}

pub struct ResponseBuilder;

impl ResponseBuilder {
    pub fn get_reply(
        reply_number: u16,
        parameters: Vec<&str>,
        trailing: Option<&str>,
    ) -> ResponseNumber {
        use ResponseNumber::*;
        let string_to_send = if trailing.is_some() {
            trailing.unwrap_or_default().to_string()
        } else {
            parameters.join(" ")
        };
        match reply_number {
            1 => Welcome(string_to_send),
            2 => YourHost(string_to_send),
            3 => Created(string_to_send),
            4 => MyInfo(string_to_send),
            5 => Bounce(string_to_send),
            200 => TraceLink(string_to_send),
            201 => TraceConnecting(string_to_send),
            202 => TraceHandshake(string_to_send),
            203 => TraceUnknown(string_to_send),
            204 => TraceOperator(string_to_send),
            205 => TraceUser(string_to_send),
            206 => TraceServer(string_to_send),
            208 => TraceService(string_to_send),
            209 => TraceNewType(string_to_send),
            210 => TraceLog(string_to_send),
            211 => StatsLinkInfo(string_to_send),
            212 => StatsCommands(string_to_send),
            213 => StatsCLine(string_to_send),
            214 => StatsNLine(string_to_send),
            215 => StatsILine(string_to_send),
            216 => StatsKLine(string_to_send),
            217 => StatsQLine(string_to_send),
            218 => StatsYLine(string_to_send),
            219 => EndOfStats(string_to_send),
            221 => UserModeIs(string_to_send),
            231 => ServiceInfo(string_to_send),
            232 => EndOfService(string_to_send),
            233 => StatsULine(string_to_send),
            234 => StatsVLine(string_to_send),
            235 => StatsXLine(string_to_send),
            241 => StatsLLine(string_to_send),
            242 => StatsUptime(string_to_send),
            243 => StatsOLine(string_to_send),
            244 => StatsHLine(string_to_send),
            245 => StatsPLine(string_to_send),
            246 => StatsDLine(string_to_send),
            247 => StatsTLine(string_to_send),
            250 => HighestConnCount(string_to_send),
            251 => LUserClient(string_to_send),
            252 => LUserOp(string_to_send),
            253 => LUserUnknown(string_to_send),
            254 => LUserChannels(string_to_send),
            255 => LUserMe(string_to_send),
            256 => AdminMe(string_to_send),
            257 => AdminLoc1(string_to_send),
            258 => AdminLoc2(string_to_send),
            259 => AdminEmail(string_to_send),
            261 => TraceLog2(string_to_send),
            262 => EndOfLUser(string_to_send),
            263 => TryAgain(string_to_send),
            265 => LocalUsers(string_to_send),
            266 => GlobalUsers(string_to_send),
            300 => None(string_to_send),
            301 => Away(string_to_send),
            302 => UserHost(string_to_send),
            303 => Ison(string_to_send),
            304 => Text(string_to_send),
            305 => UnAway(string_to_send),
            306 => NowAway(string_to_send),
            311 => WhoisUser(string_to_send),
            312 => WhoisServer(string_to_send),
            313 => WhoisOperator(string_to_send),
            314 => WhowasUser(string_to_send),
            315 => EndOfWho(string_to_send),
            316 => WhoisIdle(string_to_send),
            317 => WhoisIdleTime(string_to_send),
            318 => EndOfWhois(string_to_send),
            319 => WhoisChannels(string_to_send),
            321 => ListStart(string_to_send),
            322 => List(string_to_send),
            323 => ListEnd(string_to_send),
            324 => ChannelModeIs(string_to_send),
            325 => UniqueOpIs(string_to_send),
            331 => NoTopic(string_to_send),
            332 => Topic(string_to_send),
            333 => TopicWhoTime(parameters.join(" ")),
            341 => Invite(string_to_send),
            342 => SummonAnswer(string_to_send),
            346 => InviteList(string_to_send),
            347 => EndOfInviteList(string_to_send),
            348 => ExceptionList(string_to_send),
            349 => EndOfExceptionList(string_to_send),
            351 => Version(string_to_send),
            352 => WhoReply(string_to_send),
            353 => NameReply(
                string_to_send
                    .split_ascii_whitespace()
                    .map(|v| v.to_string())
                    .collect::<Vec<String>>(),
            ),
            354 => WhoReplyExtended(string_to_send),
            361 => KillDone(string_to_send),
            362 => Closing(string_to_send),
            363 => Links(string_to_send),
            364 => Links2(string_to_send),
            365 => EndOfLinks(string_to_send),
            366 => EndOfNames(string_to_send),
            367 => BanList(string_to_send),
            368 => EndOfBanList(string_to_send),
            369 => EndOfWhowas(string_to_send),
            371 => Info(string_to_send),
            372 => MOTD(string_to_send),
            373 => MOTDStart(string_to_send),
            374 => EndOfInfo(string_to_send),
            375 => MOTDStart2(string_to_send),
            376 => EndOfMOTD(string_to_send),
            381 => YouAreOper(string_to_send),
            382 => Rehashing(string_to_send),
            383 => YouAreService(string_to_send),
            391 => Time(string_to_send),
            392 => UsersStart(string_to_send),
            393 => Users(string_to_send),
            394 => EndOfUsers(string_to_send),
            395 => NoUsers(string_to_send),
            400..=502 | 524..=525 | 691 | 696 | 723 | 902 | 904..=907 => {
                Err(reply_number, string_to_send)
            }
            _ => Unknown(reply_number, string_to_send),
        }
    }
}
