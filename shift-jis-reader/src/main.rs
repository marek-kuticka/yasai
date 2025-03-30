use std::fs::File;
use std::io::{self, BufRead, BufReader};
use encoding_rs::{SHIFT_JIS, UTF_8};
use encoding_rs_io::DecodeReaderBytesBuilder;

use std::time::{Instant};

use std::collections::{HashMap, HashSet};
use std::ops::Index;

use std::fmt;
use std::fmt::{write, Debug, Formatter};
use uuid::Uuid;

use shogi_core::{Color, Hand, Move, PartialPosition, Piece, PieceKind, Square, ToUsi};
use yasai::Position;
use shogi_core::consts::square::{SQ_1A, SQ_1C};

pub struct DisplayableMove(pub shogi_core::Move);

impl fmt::Display for DisplayableMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {

        match &self.0  {
            Move::Normal {from, to, promote} => {
                let from = format!("{}", from.to_usi_owned());
                let to = format!("{}", to.to_usi_owned());
                let promote = if *promote { " (promote)" } else { "" };


                write!(f, "{} -> {}{}", from, to, promote)
            },
            Move::Drop { piece, to } => {
                let to = format!("{}", to.to_usi_owned());

                write!(f, "drop {} {}", DisplayablePiece(piece.clone()), to)
            }
        }

    }
}

pub struct DisplayablePiece(pub shogi_core::Piece);

impl fmt::Display for DisplayablePiece {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let piece = &self.0.piece_kind().to_usi_owned();

        write!(f, "{}", piece)
    }
}

// impl fmt::Display for DisplayableMove {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         match &self  {
//             &_ => {
//             write!(f, "NoMove")
//             }
//         }
//     }
// }


enum ParserState {
    Initial,
    MainLine,
    InVariation1,
    InVariation,
    AfterVariation,
}

struct ParserContext {
    line_num_from: usize,
    line_num_to: usize,
    move_from: usize,
    move_to: usize,

    current_seq: Vec<Move>,

    seqs: HashMap<Uuid, ShogiSequence>,

    is_initialized: bool,
    pub context_name: String,
    current_sequence: Uuid,
    main_sequence: Uuid,

    pos: Position,
}

impl ParserContext {
    // Konstruktor ParserContext
    pub fn new(context_name: &str) -> Self {
        let root = Uuid::new_v4();

        Self {
            line_num_from: 1,
            line_num_to: 4,
            move_from: 0,
            move_to: 0,
            seqs: Default::default(),
            current_seq: vec![],
            is_initialized: false,
            context_name: context_name.to_string(),
            current_sequence: root,
            main_sequence: root,
            pos: Position::new(PartialPosition::startpos())
        }
    }

    pub fn last_move(&mut self) -> (u8, u8) {
    let last_move = self.current_seq.last().unwrap();
        match last_move {
            Move::Normal {from, to, promote} => {
                (from.file(), from.rank())
            }
            _ => (0,0)
        }
    }


    pub fn create_sequence(&mut self, start_move_number: usize) -> Uuid {
        let parent = self.current_sequence;

        println!("create_sequence START with current_sequence: {}", self.current_sequence);

        let seq = ShogiSequence {
            moves: MoveVec::new(vec![], start_move_number),
            follow_ups: HashSet::new(),
            parent,
            start_move_number,
        };

        let uuid = Uuid::new_v4();
        self.seqs.insert(uuid, seq);

        self.current_sequence = uuid;
        println!("create_sequence END with new sequence: {}", uuid);

        uuid
    }

    pub fn dump_sequences(&self, msg: String) {
        println!("{} Sekvence v kontextu:", msg);
        for (uuid, seq) in &self.seqs {
            println!("uuid: {}, start_move_number: {}, moves: {}", uuid, seq.start_move_number, seq.moves.len());

            for uuid in seq.follow_ups.iter() {
                println!("  follow_up uuid: {}", uuid);
            }

            println!("  moves:");

            for (i, m) in seq.moves.moves.iter().enumerate() {
                // match m {
                //     Move::OkMove(ref m) => {
                //         println!("  {}: {}", i + seq.start_move_number, m.move_str);
                //     }
                //     Move::NoMove(_) => {}
                // }
                println!("  {}: {}", i + seq.start_move_number, DisplayableMove(m.clone()));
            }
        }

        println!("\n\n");
    }

    pub fn dump_pos(&mut self) {
        print!("");
        for rank in 1..=9 {    // Outer loop from 1 to 9
            for file in (1..=9).rev() {  // Inner loop from 9 to 1
                let piece = self.pos.piece_at(Square::new(file, rank).unwrap());

                match piece {
                    None => {
                        print!(". ")
                    }
                    Some(p) => {
                        let owner = p.color();

                        match owner {
                            Color::Black => {
                                print!("{} ", p.piece_kind().to_usi_owned().to_lowercase());
                            }
                            Color::White => {
                                print!("{} ", p.piece_kind().to_usi_owned());
                            }
                        }

                    }
                }

            }
            println!("")
        }

        println!("/");
    }

    // Přidání nového tahu do current_sequence
    pub fn add_move(&mut self, num: usize, game_move: Move) {


        if let Some(current_seq) = self.seqs.get_mut(&self.current_sequence) {
            current_seq.moves.moves.push(game_move.clone());
            //println!("adding move {} {} for sequence: {}",num, DisplayableMove(game_move), self.current_sequence);

            // add move to current_seq.

            //println!("before");

            //self.dump_pos();

            let is_legal = self.pos.legal_moves().iter().any(|m| m == &game_move);

            if !is_legal {
                println!("move {} is not legal\ncurrent pos:", DisplayableMove(game_move));
                self.dump_pos();

                println!("moves up til now");
                for (i, m) in self.current_seq.iter().enumerate() {
                    println!("Move {}: {}", i+1, DisplayableMove(m.clone()));
                }

                println!("all legal moves:");

                for m in self.pos.legal_moves() {
                    println!(" move: {}", DisplayableMove(m));
                }
            }

            self.current_seq.push(game_move);
            self.pos.do_move(game_move);

            //println!("after");
            //self.dump_pos();

            //println!("\n");



        } else {
            panic!("Aktuální sekvence není nastavena!"); // Nebo můžete použít jiný mechanismus pro správu chyby.
        }


    }

    pub fn find_root(&self, start_sequence: Uuid) -> Uuid {
        let mut current_uuid = start_sequence;

        // Iterativní procházení k rodiči
        while let Some(sequence) = self.seqs.get(&current_uuid) {
            if sequence.parent == current_uuid {
                // Našli jsme nejvyššího rodiče
                break;
            }
            current_uuid = sequence.parent; // Přechod na rodiče
        }

        current_uuid // Návrat UUID nejvyššího rodiče
    }


    pub fn add_variation(&mut self, start_move_number: usize) {

        //println!("add_variation START with starting move: {}", start_move_number);

        // finding parent of start_move_number
        let mut parent_uuid = self.current_sequence;

        let parent_move_number = start_move_number - 1;

        while let Some(seq) = self.seqs.get(&parent_uuid) {
            if parent_move_number >= seq.start_move_number && parent_move_number < seq.start_move_number + seq.moves.len() {
                break;
            }
            parent_uuid = seq.parent;
        }

        let mut seqs_temp: HashMap<Uuid, ShogiSequence> = HashMap::new();

        if let Some(mut c_s) = self.seqs.get_mut(&parent_uuid) {

            let remaining_moves = c_s.split_at_move(parent_move_number+1);

            if (remaining_moves.len() > 0) {
                let mut seq = ShogiSequence {
                    moves: MoveVec::new(remaining_moves, start_move_number),
                    follow_ups: HashSet::new(),
                    parent: parent_uuid,
                    start_move_number,
                };

                for i in c_s.follow_ups.drain() {
                    seq.follow_ups.insert(i);
                }

                let uuid = Uuid::new_v4();
                seqs_temp.insert(uuid, seq);

                c_s.follow_ups.insert(uuid);

            } else {

            }

            let seq_new = ShogiSequence {
                moves: MoveVec::new(vec![], start_move_number),
                follow_ups: HashSet::new(),
                parent: parent_uuid,
                start_move_number,
            };

            let uuid_new = Uuid::new_v4();
            seqs_temp.insert(uuid_new, seq_new);
            c_s.follow_ups.insert(uuid_new);

            self.current_sequence = uuid_new;

        } else {
            panic!("could not find parent for move {}", parent_move_number);
        }

        self.seqs.extend(seqs_temp);

        // truncate current_seq to contain only moves up to parent_move
        //self.current_seq.truncate(parent_move_number);
        let mut remaining = self.current_seq.split_off(parent_move_number);
        //println!("current_seq after split: {}, remaining moves: {}", self.current_seq.len(), remaining.len());


        //println!("current_seq before truncating has {} moves", self.pos.states_len()-1);
        for m in remaining.iter().rev() {
            //println!("removing move {} from current_seq", DisplayableMove(m.clone()));
            self.pos.undo_move(*m);
            // println!("after");
            //self.dump_pos();
            // println!("\n");
        }

        //println!("current_seq truncated to {} moves", self.pos.states_len()-1);

    }

}

#[derive(Debug, Clone)]
struct MoveInfo {
    line_num: i32,
    move_str: String
}

// #[derive(Debug, Clone)]
// enum Move {
//     OkMove(MoveInfo),
//     NoMove(String),
// }

// impl fmt::Display for Move {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Move::OkMove(mv) => {
//                 write!(f, "{}", mv.move_str)
//             }
//             Move::NoMove(_) => {
//                 write!(f, "NoMove")
//             }
//         }
//
//     }
//
// }


fn read_shift_jis_lines(filename: &str) -> io::Result<Vec<String>> {
    let file = File::open(filename)?;
    let decoder = DecodeReaderBytesBuilder::new()
        .encoding(Some(SHIFT_JIS))
        .build(file);
    let reader = BufReader::new(decoder);

    let mut lines = Vec::new();
    for line_result in reader.lines() {
        let line = line_result?;
        lines.push(line);
    }

    Ok(lines)
}

#[derive(Debug, Clone)]
struct ShogiSequence {
    moves: MoveVec<Move>,
    follow_ups: HashSet<Uuid>,
    parent: Uuid,
    start_move_number: usize,
}

impl ShogiSequence {
    fn split_at_move(&mut self, move_number: usize) -> Vec<Move> {
        let split_index = move_number - self.start_move_number;
        if split_index >= self.moves.moves.len() {
            return Vec::new(); // Není co dělit, variace nezačíná uvnitř této sekvence
        }

        // Oddělíme tahy od split_index
        let remaining_moves = self.moves.moves.split_off(split_index);

        remaining_moves
    }
}



#[derive(Debug, Clone)]
struct MoveVec<T> {
    moves: Vec<T>,
    start_move_number: usize,
}

impl<T> MoveVec<T> {
    fn new(moves: Vec<T>, start_move_number: usize) -> Self {
        MoveVec {
            moves,
            start_move_number,
        }
    }

    fn len(&self) -> usize {
        self.moves.len()
    }

    fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }
}

impl<T> Index<usize> for MoveVec<T> {
    type Output = T;

    fn index(&self, move_number: usize) -> &Self::Output {
        if move_number < self.start_move_number {
            panic!("Move number out of range: {} < {}", move_number, self.start_move_number);
        }

        let index = move_number - self.start_move_number;

        if index >= self.moves.len() {
            panic!(
                "Move number out of range: {} > {}",
                move_number,
                self.start_move_number + self.moves.len() - 1
            );
        }

        &self.moves[index]
    }
}

fn parse_coordinate(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<usize> {
    match chars.next() {
        Some('１') | Some('一') | Some('1') => Some(1),
        Some('２') | Some('二') | Some('2') => Some(2),
        Some('３') | Some('三') | Some('3') => Some(3),
        Some('４') | Some('四') | Some('4') => Some(4),
        Some('５') | Some('五') | Some('5') => Some(5),
        Some('６') | Some('六') | Some('6') => Some(6),
        Some('７') | Some('七') | Some('7') => Some(7),
        Some('８') | Some('八') | Some('8') => Some(8),
        Some('９') | Some('九') | Some('9') => Some(9),
        _ => None,
    }
}

fn parse_square(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<Square> {
    let file = parse_coordinate(chars);
    if file.is_none() {
        return None;
    }

    let rank = parse_coordinate(chars);
    if rank.is_none() {
        return None;
    }

    Square::new(file.unwrap() as u8, rank.unwrap() as u8)
}

fn char_to_coord(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<usize>{
    match chars.next() {
        Some('1') => Some(1),
        Some('2') => Some(2),
        Some('3') => Some(3),
        Some('4') => Some(4),
        Some('5') => Some(5),
        Some('6') => Some(6),
        Some('7') => Some(7),
        Some('8') => Some(8),
        Some('9') => Some(9),
        _ => None,
    }
}

fn parse_piece(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<PieceKind> {
    
    match chars.next() {

        Some('成') => {
            // return already promoted stone...
            match chars.next() {
                Some('香') => Some(PieceKind::ProLance),
                Some('桂') => Some(PieceKind::ProKnight),
                Some('銀') => Some(PieceKind::ProSilver),
                _ => {
                    println!("unknown char " );
                    None
                },
            }
        },
        Some('歩')  => Some(PieceKind::Pawn),
        Some('と') => Some(PieceKind::ProPawn),
        Some('香') => Some(PieceKind::Lance),   // 香車
        Some('桂') => Some(PieceKind::Knight), // 桂馬
        Some('銀') => Some(PieceKind::Silver), // 銀将
        Some('金') => Some(PieceKind::Gold),   // 金将
        Some('角') => Some(PieceKind::Bishop), // 角行
        Some('馬') => Some(PieceKind::ProBishop), // 角行
        Some('飛') => Some(PieceKind::Rook),   // 飛車
        Some('龍') | Some('竜') => Some(PieceKind::ProRook),   // 飛車
        Some('王') | Some('玉') => Some(PieceKind::King),



        Some(c) => {
            println!("Unknown char: {}", c);
            None
        },
        _ => {
            None
        }
    }

}

fn parse_promote(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<bool> {
    if chars.peek() == Some(&'成') {
        chars.next();
        Some(true)
    } else {
        Some(false)
    }
}


fn parse_number_from_line(line: &str, ctx: &ParserContext) -> (usize, Option<Move>) {
    if line.len() < 4 {
        return (0, Option::None);
    }

    let str_num =
        line[ctx.line_num_from .. ctx.line_num_to]
        .trim()
        .parse::<i32>();

    if let Ok(l_num) = str_num {

        let split =
            line[ctx.line_num_to ..]
                .split("(")
                .map(|s| s.trim())
                .collect::<Vec<&str>>();

        let move_info = MoveInfo {
            line_num: l_num, // Použijeme ukazatel na číslo řádky
            move_str: split[0].to_string(), // Předpokládáme, že řádek reprezentuje tah
        };
        // let new_move = Move::Normal {
        //     from: SQ_1A,
        //     to: SQ_1C,
        //     promote: false,
        // };



        //let x = mv_data.chars().nth(0).unwrap().to_digit(10)? as u8;
        //let y = mv_data.chars().nth(1)?.to_digit(10)? as u8;

        let mv_data = split[0].trim();

        let new_move = match split.iter().len() {
            3 => {
                //let x = mv_data.chars().nth(0).unwrap().to_digit(10)? as u8;
                //println!("parsing move: <{}>", split[0].trim());

                let mut pk = split[0].trim().chars().peekable();

                let sq_to: Option<Square> =
                    if pk.peek() == Some(&'同') {
                        pk.next();
                        pk.next();
                        let last = ctx.current_seq.last().unwrap().to();
                        //println!("Using last move as target: {}", last.to_usi_owned());
                        Some(last)
                    } else {
                        parse_square(&mut pk)
                    };



                if sq_to.is_none() {
                    println!("Could not parse to square from line {}", line);
                    return (0, Option::None);
                };

                let piece_to = parse_piece(&mut pk);
                let promote = parse_promote(&mut pk).unwrap_or(false);
                if promote {
                    //println!("Promote found");
                }

                let mut pk_from = split[1].trim().chars().peekable();
                let sq_from = parse_square(&mut pk_from);

                if sq_from.is_none() {
                    return (0, Option::None);
                }


                Move::Normal {
                    from: sq_from.unwrap(),
                    to: sq_to.unwrap(),
                    promote: promote,
                }
            },

            2 => {
                if mv_data.ends_with("打") {
                    //println!("parsing move: <{}>", split[0].trim());
                    let mut pk = split[0].trim().chars().peekable();
                    let sq_to = parse_square(&mut pk);

                    if sq_to.is_none() {
                        return (0, Option::None);
                    }

                    let piece = parse_piece(&mut pk);

                    if piece.is_none() {
                        return (0, Option::None);
                    }

                    Move::Drop {
                        piece: Piece::new(piece.unwrap(), ctx.pos.side_to_move()),
                        to: sq_to.unwrap(),
                    }
                } else {
                    return (0, Option::None)
                }
            },
            _ => {
                return (0, Option::None);
            }

        };


        // match mv_data.chars().next() {
        //     Some(c @'０'..='９') =>
        //         {}
        //     Some('同') => {}
        //     _ => {}
        // }


        //println!("split: {:?}", split);






        (l_num as usize, Some(new_move))
    } else {
        (0, Option::None)
        //panic!("Could not parse number from line {}", line)
    }

}

fn create_context(ctx: &mut ParserContext, line: &str)  {
    ctx.line_num_from = 1;
    ctx.line_num_to = 4;
    ctx.is_initialized = true;

    let uuid = Uuid::new_v4();

    ctx.current_sequence = uuid; //Some(main_sequence.clone());
    ctx.main_sequence = uuid;
}

fn main() -> io::Result<()> {
    // Replace "your_file.txt" with the actual path to your Shift-JIS file.
    let filename = "/Users/marek/RustroverProjects/yasai2/shift-jis-reader/kakugawari.kif";

    let mut context = ParserContext::new("main");
    context.create_sequence(1 );

    println!("Reading file: {}", filename);

    let start = Instant::now();

    match read_shift_jis_lines(filename) {
        Ok(lines) => {

            let mut parser_state = ParserState::Initial;
            for (line_num, line) in lines.iter().enumerate() {
                //println!("parsing line: {} - {}", line_num+1, line);

                if line.starts_with("*") {
                    continue;
                }

                match parser_state {
                    ParserState::Initial => {
                        if line.starts_with("手数") {
                            parser_state = ParserState::MainLine;
                            println!("Main line at line {}", line_num + 1);
                        }

                        if line.starts_with("変化") {
                            parser_state = ParserState::InVariation1;
                        }
                    }
                    ParserState::MainLine => {
                        if line.is_empty() {
                            parser_state = ParserState::Initial;
                            //println!("back to initial at line {}", line_num+1);
                        } else {
                            let (num, mv) = parse_number_from_line(line, &context);
                            match mv {
                                Some(m) => {
                                    context.add_move(num,m);
                                }
                                None => {}
                            }

                        }

                    }

                    ParserState::InVariation1 => {
                        parser_state = ParserState::InVariation;

                        let  (num, _) = parse_number_from_line(line, &context);
                        context.add_variation(num);

                        let (_ , mv) = parse_number_from_line(line, &context);
                        match mv {
                            Some(m) => {
                                context.add_move(num,m);
                            }
                            None => {}
                        }

                    }

                    ParserState::InVariation => {
                        if line.is_empty() {
                            parser_state = ParserState::Initial;

                        } else {
                            let (num,mv) = parse_number_from_line(line, &context);
                            match mv {
                                Some(m) => {
                                    context.add_move(num,m);
                                }
                                None => {}
                            }

                        }

                    }
                    ParserState::AfterVariation => {}
                }
            }
        }
        Err(err) => {
            eprintln!("Error reading file: {}", err);
        }
    }
    println!("Parsed file: {}", filename);
    let duration = start.elapsed();
    println!("Program executed in: {:?}", duration);

    //context.dump_sequences(String::from("main"));
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number_from_line() {

        let uuid = Uuid::new_v4();
        let mut context: ParserContext = ParserContext {
            line_num_from: 0,
            line_num_to: 0,
            move_from: 0,
            move_to: 0,

            current_seq: vec![],
            seqs: Default::default(),
            is_initialized: false,
            context_name: String::from("main"),
            current_sequence: uuid,
            main_sequence: uuid,
            pos: Position::new(PartialPosition::startpos())
        };

        fn sq (file: u8, rank: u8) -> Square {
            Square::new(file,rank).unwrap()
        }

        //create_context(&mut context, "1234567890");
        context.create_sequence(1);

        context.add_move(Move::Normal {from: sq(1,3), to: sq(1,4), promote: false });

        context.add_move(Move::OkMove(MoveInfo {
            line_num: 1,
            move_str: "M1".to_string(),
        }));

        context.add_move(Move::OkMove(MoveInfo {
            line_num: 2,
            move_str: "M2".to_string(),
        }));

        context.add_move(Move::OkMove(MoveInfo {
            line_num: 3,
            move_str: "M3".to_string(),
        }));

        context.add_move(Move::OkMove(MoveInfo {
            line_num: 4,
            move_str: "M4".to_string(),
        }));


        if let Some(c_sequence) = context.seqs.get(&context.current_sequence) {
            println!("Current seq has {} moves", c_sequence.moves.len());
        }

        context.dump_sequences(String::from("main"));

        context.add_variation(3);
        context.add_move(Move::OkMove(MoveInfo {
            line_num: 3,
            move_str: "M3v".to_string(),
        }));

        context.dump_sequences(String::from("after v1 from move 3"));

        context.add_variation(2);
        context.add_move(Move::OkMove(MoveInfo {
            line_num: 2,
            move_str: "M2vv".to_string(),
        }));
        context.add_move(Move::OkMove(MoveInfo {
            line_num: 3,
            move_str: "M3vv".to_string(),
        }));
        context.add_move(Move::OkMove(MoveInfo {
            line_num: 4,
            move_str: "M4vv".to_string(),
        }));

        context.dump_sequences(String::from("after v2 from move 2"));

    }
}