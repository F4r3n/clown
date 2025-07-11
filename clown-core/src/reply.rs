use crate::command::Command;

#[derive(Debug)]
pub enum Reply {
    Cmd(Command),
    Rpl(ReplyNumber),
}
/// All standard IRC RPL (Reply) numerics.
/// See: RFC 1459, RFC 2812
#[derive(Debug)]
pub enum ReplyNumber {
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
    NameReply(String),
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

    /// Any other reply not explicitly listed
    Unknown(u16, String),
}

pub struct ReplyBuilder;

impl ReplyBuilder {
    fn make_reply_1<F>(parameters: Vec<&str>, ctor: F) -> Option<ReplyNumber>
    where
        F: Fn(String) -> ReplyNumber,
    {
        let msg = parameters.join(" ");
        if msg.is_empty() {
            None
        } else {
            Some(ctor(msg))
        }
    }

    pub fn get_reply(reply_number: u16, trailing: Vec<&str>) -> Option<ReplyNumber> {
        use ReplyNumber::*;
        match reply_number {
            1 => Self::make_reply_1(trailing, Welcome),
            2 => Self::make_reply_1(trailing, YourHost),
            3 => Self::make_reply_1(trailing, Created),
            4 => Self::make_reply_1(trailing, MyInfo),
            5 => Self::make_reply_1(trailing, Bounce),
            200 => Self::make_reply_1(trailing, TraceLink),
            201 => Self::make_reply_1(trailing, TraceConnecting),
            202 => Self::make_reply_1(trailing, TraceHandshake),
            203 => Self::make_reply_1(trailing, TraceUnknown),
            204 => Self::make_reply_1(trailing, TraceOperator),
            205 => Self::make_reply_1(trailing, TraceUser),
            206 => Self::make_reply_1(trailing, TraceServer),
            208 => Self::make_reply_1(trailing, TraceService),
            209 => Self::make_reply_1(trailing, TraceNewType),
            210 => Self::make_reply_1(trailing, TraceLog),
            211 => Self::make_reply_1(trailing, StatsLinkInfo),
            212 => Self::make_reply_1(trailing, StatsCommands),
            213 => Self::make_reply_1(trailing, StatsCLine),
            214 => Self::make_reply_1(trailing, StatsNLine),
            215 => Self::make_reply_1(trailing, StatsILine),
            216 => Self::make_reply_1(trailing, StatsKLine),
            217 => Self::make_reply_1(trailing, StatsQLine),
            218 => Self::make_reply_1(trailing, StatsYLine),
            219 => Self::make_reply_1(trailing, EndOfStats),
            221 => Self::make_reply_1(trailing, UserModeIs),
            231 => Self::make_reply_1(trailing, ServiceInfo),
            232 => Self::make_reply_1(trailing, EndOfService),
            233 => Self::make_reply_1(trailing, StatsULine),
            234 => Self::make_reply_1(trailing, StatsVLine),
            235 => Self::make_reply_1(trailing, StatsXLine),
            241 => Self::make_reply_1(trailing, StatsLLine),
            242 => Self::make_reply_1(trailing, StatsUptime),
            243 => Self::make_reply_1(trailing, StatsOLine),
            244 => Self::make_reply_1(trailing, StatsHLine),
            245 => Self::make_reply_1(trailing, StatsPLine),
            246 => Self::make_reply_1(trailing, StatsDLine),
            247 => Self::make_reply_1(trailing, StatsTLine),
            250 => Self::make_reply_1(trailing, HighestConnCount),
            251 => Self::make_reply_1(trailing, LUserClient),
            252 => Self::make_reply_1(trailing, LUserOp),
            253 => Self::make_reply_1(trailing, LUserUnknown),
            254 => Self::make_reply_1(trailing, LUserChannels),
            255 => Self::make_reply_1(trailing, LUserMe),
            256 => Self::make_reply_1(trailing, AdminMe),
            257 => Self::make_reply_1(trailing, AdminLoc1),
            258 => Self::make_reply_1(trailing, AdminLoc2),
            259 => Self::make_reply_1(trailing, AdminEmail),
            261 => Self::make_reply_1(trailing, TraceLog2),
            262 => Self::make_reply_1(trailing, EndOfLUser),
            263 => Self::make_reply_1(trailing, TryAgain),
            265 => Self::make_reply_1(trailing, LocalUsers),
            266 => Self::make_reply_1(trailing, GlobalUsers),
            300 => Self::make_reply_1(trailing, None),
            301 => Self::make_reply_1(trailing, Away),
            302 => Self::make_reply_1(trailing, UserHost),
            303 => Self::make_reply_1(trailing, Ison),
            304 => Self::make_reply_1(trailing, Text),
            305 => Self::make_reply_1(trailing, UnAway),
            306 => Self::make_reply_1(trailing, NowAway),
            311 => Self::make_reply_1(trailing, WhoisUser),
            312 => Self::make_reply_1(trailing, WhoisServer),
            313 => Self::make_reply_1(trailing, WhoisOperator),
            314 => Self::make_reply_1(trailing, WhowasUser),
            315 => Self::make_reply_1(trailing, EndOfWho),
            316 => Self::make_reply_1(trailing, WhoisIdle),
            317 => Self::make_reply_1(trailing, WhoisIdleTime),
            318 => Self::make_reply_1(trailing, EndOfWhois),
            319 => Self::make_reply_1(trailing, WhoisChannels),
            321 => Self::make_reply_1(trailing, ListStart),
            322 => Self::make_reply_1(trailing, List),
            323 => Self::make_reply_1(trailing, ListEnd),
            324 => Self::make_reply_1(trailing, ChannelModeIs),
            325 => Self::make_reply_1(trailing, UniqueOpIs),
            331 => Self::make_reply_1(trailing, NoTopic),
            332 => Self::make_reply_1(trailing, Topic),
            333 => Self::make_reply_1(trailing, TopicWhoTime),
            341 => Self::make_reply_1(trailing, Invite),
            342 => Self::make_reply_1(trailing, SummonAnswer),
            346 => Self::make_reply_1(trailing, InviteList),
            347 => Self::make_reply_1(trailing, EndOfInviteList),
            348 => Self::make_reply_1(trailing, ExceptionList),
            349 => Self::make_reply_1(trailing, EndOfExceptionList),
            351 => Self::make_reply_1(trailing, Version),
            352 => Self::make_reply_1(trailing, WhoReply),
            353 => Self::make_reply_1(trailing, NameReply),
            354 => Self::make_reply_1(trailing, WhoReplyExtended),
            361 => Self::make_reply_1(trailing, KillDone),
            362 => Self::make_reply_1(trailing, Closing),
            363 => Self::make_reply_1(trailing, Links),
            364 => Self::make_reply_1(trailing, Links2),
            365 => Self::make_reply_1(trailing, EndOfLinks),
            366 => Self::make_reply_1(trailing, EndOfNames),
            367 => Self::make_reply_1(trailing, BanList),
            368 => Self::make_reply_1(trailing, EndOfBanList),
            369 => Self::make_reply_1(trailing, EndOfWhowas),
            371 => Self::make_reply_1(trailing, Info),
            372 => Self::make_reply_1(trailing, MOTD),
            373 => Self::make_reply_1(trailing, MOTDStart),
            374 => Self::make_reply_1(trailing, EndOfInfo),
            375 => Self::make_reply_1(trailing, MOTDStart2),
            376 => Self::make_reply_1(trailing, EndOfMOTD),
            381 => Self::make_reply_1(trailing, YouAreOper),
            382 => Self::make_reply_1(trailing, Rehashing),
            383 => Self::make_reply_1(trailing, YouAreService),
            391 => Self::make_reply_1(trailing, Time),
            392 => Self::make_reply_1(trailing, UsersStart),
            393 => Self::make_reply_1(trailing, Users),
            394 => Self::make_reply_1(trailing, EndOfUsers),
            395 => Self::make_reply_1(trailing, NoUsers),
            other => Some(Unknown(other, trailing.join(" "))),
        }
    }
}
