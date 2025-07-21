use core::fmt;

use crate::command::Command;

#[derive(Debug)]
pub enum Response {
    Cmd(Command),
    Rpl(ResponseNumber),
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Response::Cmd(cmd) => {
                write!(f, "{:?}", cmd)
            }
            Response::Rpl(rpl) => {
                write!(f, "{:?}", rpl)
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
    pub fn get_reply(reply_number: u16, trailing: Option<&str>) -> Option<ResponseNumber> {
        use ResponseNumber::*;
        match reply_number {
            1 => trailing.map(|v| Welcome(v.to_string())),
            2 => trailing.map(|v| YourHost(v.to_string())),
            3 => trailing.map(|v| Created(v.to_string())),
            4 => trailing.map(|v| MyInfo(v.to_string())),
            5 => trailing.map(|v| Bounce(v.to_string())),
            200 => trailing.map(|v| TraceLink(v.to_string())),
            201 => trailing.map(|v| TraceConnecting(v.to_string())),
            202 => trailing.map(|v| TraceHandshake(v.to_string())),
            203 => trailing.map(|v| TraceUnknown(v.to_string())),
            204 => trailing.map(|v| TraceOperator(v.to_string())),
            205 => trailing.map(|v| TraceUser(v.to_string())),
            206 => trailing.map(|v| TraceServer(v.to_string())),
            208 => trailing.map(|v| TraceService(v.to_string())),
            209 => trailing.map(|v| TraceNewType(v.to_string())),
            210 => trailing.map(|v| TraceLog(v.to_string())),
            211 => trailing.map(|v| StatsLinkInfo(v.to_string())),
            212 => trailing.map(|v| StatsCommands(v.to_string())),
            213 => trailing.map(|v| StatsCLine(v.to_string())),
            214 => trailing.map(|v| StatsNLine(v.to_string())),
            215 => trailing.map(|v| StatsILine(v.to_string())),
            216 => trailing.map(|v| StatsKLine(v.to_string())),
            217 => trailing.map(|v| StatsQLine(v.to_string())),
            218 => trailing.map(|v| StatsYLine(v.to_string())),
            219 => trailing.map(|v| EndOfStats(v.to_string())),
            221 => trailing.map(|v| UserModeIs(v.to_string())),
            231 => trailing.map(|v| ServiceInfo(v.to_string())),
            232 => trailing.map(|v| EndOfService(v.to_string())),
            233 => trailing.map(|v| StatsULine(v.to_string())),
            234 => trailing.map(|v| StatsVLine(v.to_string())),
            235 => trailing.map(|v| StatsXLine(v.to_string())),
            241 => trailing.map(|v| StatsLLine(v.to_string())),
            242 => trailing.map(|v| StatsUptime(v.to_string())),
            243 => trailing.map(|v| StatsOLine(v.to_string())),
            244 => trailing.map(|v| StatsHLine(v.to_string())),
            245 => trailing.map(|v| StatsPLine(v.to_string())),
            246 => trailing.map(|v| StatsDLine(v.to_string())),
            247 => trailing.map(|v| StatsTLine(v.to_string())),
            250 => trailing.map(|v| HighestConnCount(v.to_string())),
            251 => trailing.map(|v| LUserClient(v.to_string())),
            252 => trailing.map(|v| LUserOp(v.to_string())),
            253 => trailing.map(|v| LUserUnknown(v.to_string())),
            254 => trailing.map(|v| LUserChannels(v.to_string())),
            255 => trailing.map(|v| LUserMe(v.to_string())),
            256 => trailing.map(|v| AdminMe(v.to_string())),
            257 => trailing.map(|v| AdminLoc1(v.to_string())),
            258 => trailing.map(|v| AdminLoc2(v.to_string())),
            259 => trailing.map(|v| AdminEmail(v.to_string())),
            261 => trailing.map(|v| TraceLog2(v.to_string())),
            262 => trailing.map(|v| EndOfLUser(v.to_string())),
            263 => trailing.map(|v| TryAgain(v.to_string())),
            265 => trailing.map(|v| LocalUsers(v.to_string())),
            266 => trailing.map(|v| GlobalUsers(v.to_string())),
            300 => trailing.map(|v| None(v.to_string())),
            301 => trailing.map(|v| Away(v.to_string())),
            302 => trailing.map(|v| UserHost(v.to_string())),
            303 => trailing.map(|v| Ison(v.to_string())),
            304 => trailing.map(|v| Text(v.to_string())),
            305 => trailing.map(|v| UnAway(v.to_string())),
            306 => trailing.map(|v| NowAway(v.to_string())),
            311 => trailing.map(|v| WhoisUser(v.to_string())),
            312 => trailing.map(|v| WhoisServer(v.to_string())),
            313 => trailing.map(|v| WhoisOperator(v.to_string())),
            314 => trailing.map(|v| WhowasUser(v.to_string())),
            315 => trailing.map(|v| EndOfWho(v.to_string())),
            316 => trailing.map(|v| WhoisIdle(v.to_string())),
            317 => trailing.map(|v| WhoisIdleTime(v.to_string())),
            318 => trailing.map(|v| EndOfWhois(v.to_string())),
            319 => trailing.map(|v| WhoisChannels(v.to_string())),
            321 => trailing.map(|v| ListStart(v.to_string())),
            322 => trailing.map(|v| List(v.to_string())),
            323 => trailing.map(|v| ListEnd(v.to_string())),
            324 => trailing.map(|v| ChannelModeIs(v.to_string())),
            325 => trailing.map(|v| UniqueOpIs(v.to_string())),
            331 => trailing.map(|v| NoTopic(v.to_string())),
            332 => trailing.map(|v| Topic(v.to_string())),
            333 => trailing.map(|v| TopicWhoTime(v.to_string())),
            341 => trailing.map(|v| Invite(v.to_string())),
            342 => trailing.map(|v| SummonAnswer(v.to_string())),
            346 => trailing.map(|v| InviteList(v.to_string())),
            347 => trailing.map(|v| EndOfInviteList(v.to_string())),
            348 => trailing.map(|v| ExceptionList(v.to_string())),
            349 => trailing.map(|v| EndOfExceptionList(v.to_string())),
            351 => trailing.map(|v| Version(v.to_string())),
            352 => trailing.map(|v| WhoReply(v.to_string())),
            353 => trailing.map(|v| {
                NameReply(
                    v.split_ascii_whitespace()
                        .map(|v| v.to_string())
                        .collect::<Vec<String>>(),
                )
            }),
            354 => trailing.map(|v| WhoReplyExtended(v.to_string())),
            361 => trailing.map(|v| KillDone(v.to_string())),
            362 => trailing.map(|v| Closing(v.to_string())),
            363 => trailing.map(|v| Links(v.to_string())),
            364 => trailing.map(|v| Links2(v.to_string())),
            365 => trailing.map(|v| EndOfLinks(v.to_string())),
            366 => trailing.map(|v| EndOfNames(v.to_string())),
            367 => trailing.map(|v| BanList(v.to_string())),
            368 => trailing.map(|v| EndOfBanList(v.to_string())),
            369 => trailing.map(|v| EndOfWhowas(v.to_string())),
            371 => trailing.map(|v| Info(v.to_string())),
            372 => trailing.map(|v| MOTD(v.to_string())),
            373 => trailing.map(|v| MOTDStart(v.to_string())),
            374 => trailing.map(|v| EndOfInfo(v.to_string())),
            375 => trailing.map(|v| MOTDStart2(v.to_string())),
            376 => trailing.map(|v| EndOfMOTD(v.to_string())),
            381 => trailing.map(|v| YouAreOper(v.to_string())),
            382 => trailing.map(|v| Rehashing(v.to_string())),
            383 => trailing.map(|v| YouAreService(v.to_string())),
            391 => trailing.map(|v| Time(v.to_string())),
            392 => trailing.map(|v| UsersStart(v.to_string())),
            393 => trailing.map(|v| Users(v.to_string())),
            394 => trailing.map(|v| EndOfUsers(v.to_string())),
            395 => trailing.map(|v| NoUsers(v.to_string())),
            400..=502 | 524..=525 | 691 | 696 | 723 | 902 | 904..=907 => {
                trailing.map(|v| Err(reply_number, v.to_string()))
            }
            _ => trailing.map(|v| Unknown(reply_number, v.to_string())),
        }
    }
}
