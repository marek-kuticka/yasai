use std::fs::File;
use std::io::{self, BufRead, BufReader};
use encoding_rs::SHIFT_JIS;
use encoding_rs_io::DecodeReaderBytesBuilder;

use std::time::{Instant};

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ops::Index;
use std::rc::{Rc, Weak};

use std::fmt;

use uuid::Uuid;


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

    seqs: HashMap<Uuid, ShogiSequence>,

    is_initialized: bool,
    pub context_name: String,
    current_sequence: Uuid,
    main_sequence: Uuid,
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
            is_initialized: false,
            context_name: context_name.to_string(),
            current_sequence: root,
            main_sequence: root,
        }
    }

    // Nastavení aktuální sekvence v kontextu
    // pub fn set_current_sequence(&mut self, sequence: Rc<RefCell<ShogiSequence>>) {
    //     self.current_sequence = Some(sequence);
    // }

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
                match m {
                    Move::OkMove(ref m) => {
                        println!("  {}: {}", i + seq.start_move_number, m.move_str);
                    }
                    Move::NoMove(_) => {}
                }
            }
        }

        println!("\n\n");
    }

    // Přidání nového tahu do current_sequence
    pub fn add_move(&mut self, game_move: Move) {


        if let Some(current_seq) = self.seqs.get_mut(&self.current_sequence) {
            match game_move {
                Move::OkMove(ref m) => {
                    current_seq.moves.moves.push(game_move.clone());
                    //println!("add_move {} for sequence: {}", m.move_str, self.current_sequence);
                }
                Move::NoMove(_) => {}
            }


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

        // finding parent of start_move_number
        let mut parent_uuid = self.current_sequence;

        let parent_move_number = start_move_number - 1;

        //println!("New variation starting at move {}", start_move_number);

        while let Some(seq) = self.seqs.get(&parent_uuid) {
            //println!("looking for parent for move {} in seq {}-{}", parent_move_number, seq.start_move_number, seq.start_move_number + seq.moves.len() -1);
            if parent_move_number >= seq.start_move_number && parent_move_number < seq.start_move_number + seq.moves.len() {
                //println!("found parent for move {} containing moves {}-{} in {}", parent_move_number, seq.start_move_number, seq.start_move_number + seq.moves.len()-1, parent_uuid);
                break;
            }
            parent_uuid = seq.parent;
        }

        let mut seqs_temp: HashMap<Uuid, ShogiSequence> = HashMap::new();

        if let Some(mut c_s) = self.seqs.get_mut(&parent_uuid) {
            //println!("found parent for move {} containing moves {}-{}", parent_move_number, c_s.start_move_number, c_s.start_move_number + c_s.moves.len() -1);

            //println!("splitting parent at move {}", parent_move_number+1);

            // if split move is last move of sequence -> no need to split
            // if c_s.start_move_number + c_s.moves.len() -1 < parent_move_number {}
            let remaining_moves = c_s.split_at_move(parent_move_number+1);

            //println!("old sequence after split: {}-{}", c_s.start_move_number, c_s.start_move_number + c_s.moves.len() -1);

            if (remaining_moves.len() > 0) {
                let mut seq = ShogiSequence {
                    moves: MoveVec::new(remaining_moves, start_move_number),
                    follow_ups: HashSet::new(),
                    parent: parent_uuid,
                    start_move_number,
                };

                //let old_follow_ups = c_s.follow_ups.drain();
                for i in c_s.follow_ups.drain() {
                    seq.follow_ups.insert(i);
                }
                //println!("new sequence after split: {}-{}", seq.start_move_number, seq.start_move_number + seq.moves.len() -1);

                let uuid = Uuid::new_v4();
                seqs_temp.insert(uuid, seq);

                c_s.follow_ups.insert(uuid);
                //uuids.insert(uuid);
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
            //println!("could not find parent for move {}", parent_move_number);
        }

        self.seqs.extend(seqs_temp);

    }


    // Funkce pro výpis detailů kontextu
    // pub fn display_context_details(&self) {
    //     println!("Název kontextu: {}", self.context_name);
    //
    //     if let Some(current_seq) = &self.main_sequence {
    //         let seq = current_seq.borrow();
    //         println!("Aktuální sekvence:");
    //         println!("  Startovní číslo tahu: {}", seq.start_move_number);
    //
    //         if seq.moves.is_empty() {
    //             println!("  Tahy: Žádné tahy nebyly provedeny.");
    //         } else {
    //             println!("  Tahy:");
    //             for (i, m) in seq.moves.moves.iter().enumerate() {
    //                 println!("    {}: {}", i + seq.start_move_number, m);
    //             }
    //         }
    //
    //         seq.follow_ups.iter().for_each(|(k, v)| {
    //             println!("  Variace: {}", k);
    //             let vv = v.borrow();
    //             for (i, m) in vv.moves.moves.iter().enumerate() {
    //                 println!("    {}: {}", i + vv.start_move_number, m);
    //             }
    //
    //
    //
    //         })
    //
    //
    //     } else {
    //         println!("Aktuální sekvence není nastavena.");
    //     }
    //
    //
    // }

}



fn display_context_details(sequence: &Rc<RefCell<ShogiSequence>>, depth: usize) {
    let indent = "  |".repeat(depth); // To indicate the hierarchy visually
    let seq = sequence.borrow();


    println!("{}Startovní číslo tahu: {}", indent, seq.start_move_number);

    if seq.moves.is_empty() {
        println!("{}Tahy: Žádné tahy nebyly provedeny.", indent);
    } else {
        println!("{}Tahy:", indent);
        for (i, m) in seq.moves.moves.iter().enumerate() {
            println!("{}{}: {}",indent, i + seq.start_move_number, m);
        }
    }

    // Recursively display details of follow-ups
    // for (key, follow_up) in &seq.follow_ups {
    //     println!("{}Follow-up key: {}", indent, key);
    //     //display_context_details(follow_up, depth + 1); // Recursion for nested follow-ups
    // }
}





#[derive(Debug, Clone)]
struct MoveInfo {
    line_num: i32,
    move_str: String
}

#[derive(Debug, Clone)]
enum Move {
    OkMove(MoveInfo),
    NoMove(String),
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Move::OkMove(mv) => {
                write!(f, "{}", mv.move_str)
            }
            Move::NoMove(_) => {
                write!(f, "NoMove")
            }
        }


        // match self {
        //     Move::OkMove { ln: i32, move_str, .. } => {
        //         let from_position = if let Some((x, y)) = from {
        //             format!("{}-{}", x, y) // Výchozí pozice
        //         } else {
        //             "-".to_string() // Pokud není výchozí pozice (např. "drop move")
        //         };
        //         let to_position = format!("{}-{}", to.0, to.1); // Cílová pozice
        //         if *promotion {
        //             write!(f, "{}: {} -> {} (Promotion)", piece, from_position, to_position)
        //         } else {
        //             write!(f, "{}: {} -> {}", piece, from_position, to_position)
        //         }
        //     }
        //     Move::NoMove => write!(f, "NoMove: Neprovedený tah"),
        // }
    }

}


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

// fn create_sequence(
//     moves: Vec<Move>,
//     parent: Weak<RefCell<ShogiSequence>>,
//     start_move_number: usize,
// ) -> Rc<RefCell<ShogiSequence>> {
//     Rc::new(RefCell::new(ShogiSequence {
//         moves: MoveVec::new(moves, start_move_number),
//         follow_ups: HashMap::new(),
//         parent,
//         start_move_number,
//     }))
// }

// fn add_follow_up(
//     parent: Rc<RefCell<ShogiSequence>>,
//     follow_up_moves: Vec<Move>,
//     key: String,
//     start_move_number: usize,
// ) -> Rc<RefCell<ShogiSequence>> {
//     let follow_up = create_sequence(follow_up_moves, Rc::downgrade(&parent), start_move_number);
//     parent.borrow_mut().follow_ups.insert(key, follow_up.clone());
//     follow_up
// }

// fn create_variation(
//     ctx: &mut ParserContext,
//     variation_start_move: usize,
//     variation_moves: Vec<Move>,
//     new_key: String,
// ) {
//
//     println!("create_variation START");
//
//     if let Some(current_sequence) = &ctx.current_sequence {
//         let mut parent_sequence = Rc::clone(&current_sequence);
//
//         // Najdeme rodiče správné sekvence
//         loop {
//             let sequence = parent_sequence.borrow();
//             if sequence.start_move_number <= variation_start_move
//                 && variation_start_move < sequence.start_move_number + sequence.moves.len()
//             {
//                 // Správná sekvence nalezena
//                 break;
//             }
//
//             // Posuneme se k rodiči
//             if let Some(parent) = sequence.parent.upgrade() {
//                 drop(sequence);
//                 parent_sequence = parent;
//             } else {
//                 eprintln!("Chyba: Nebylo možné najít rodičovskou sekvenci obsahující tah {}", variation_start_move);
//                 return;
//             }
//         }
//         // Nyní máme správného rodiče v `parent_sequence`
//
//         // split should be performed, only if starting move of variation is not
//         let should_split = {
//             let sequence = parent_sequence.borrow();
//
//             variation_start_move > sequence.start_move_number
//                 && variation_start_move < sequence.start_move_number + sequence.moves.len()
//         };
//
//         if should_split {
//             println!("parent seq number of moves: {}", parent_sequence.borrow_mut().moves.len() );
//
//             let mut parent = parent_sequence.borrow_mut();
//
//             let remaining_moves = {
//                 //let mut sequence = parent_sequence.borrow_mut();
//                 parent.split_at_move(variation_start_move)
//             };
//
//             println!("parent seq number of moves adfter split: {}", parent.moves.len() );
//             println!("parent seq number of moves: {}", remaining_moves.len() );
//
//             let new_variation = create_sequence(
//                 remaining_moves.clone(),
//                 Rc::downgrade(&parent_sequence),
//                 variation_start_move,
//             );
//
//             //let parent = parent_sequence.clone();
//             println!("parent current amount of follow_ups: {}", parent.follow_ups.len() );
//
//             for k in parent.follow_ups.keys() {
//                 println!("parent follow_up key: {}", k );
//             }
//
//             let tempMap: HashMap<String, Rc<RefCell< crate::ShogiSequence >>> =
//                 parent
//                     .follow_ups
//                     .drain()
//                     .collect();
//
//             let key = match new_variation.borrow().moves.moves.first().unwrap() {
//                 Move::OkMove(mv) => format!("{}", mv.move_str),
//                 _ => "".to_string()
//             };
//
//             parent
//                 .follow_ups
//                 .insert(key, new_variation.clone());
//
//             new_variation
//                 .borrow_mut()
//                 .follow_ups
//                 .extend(tempMap);
//
//             let new_seq = create_sequence(
//                 Vec::new(),
//                 Rc::downgrade(&parent_sequence),
//                 variation_start_move
//             );
//
//             parent
//                 .follow_ups
//                 .insert(new_key, new_seq.clone());
//
//             ctx.current_sequence = Some(new_seq);
//         } else {
//             let new_variation = create_sequence(
//                 variation_moves.clone(),
//                 Rc::downgrade(&parent_sequence),
//                 variation_start_move,
//             );
//
//             let key = format!("variation_at_move_{}", variation_start_move);
//             let follow_up = create_sequence(variation_moves, Rc::downgrade(&parent_sequence), variation_start_move);
//             parent_sequence.borrow_mut().follow_ups.insert(key, follow_up.clone());
//             follow_up;
//
//             ctx.current_sequence = Some(new_variation);
//         }
//     } else {
//         eprintln!("Chyba: Neexistuje aktuální sekvence pro vytvoření variace!");
//     }
//
//     println!("create_variation END");
// }


fn parse_number_from_line(line: &str, ctx: &ParserContext) -> Move {
    if line.len() < 4 {
        return Move::NoMove("".to_string() )
    }

    let str_num =
        line[ctx.line_num_from .. ctx.line_num_to]
        .trim()
        .parse::<i32>();

    if let Ok(l_num) = str_num {
        //println!("line: {}", l_num);

        let move_info = MoveInfo {
            line_num: l_num, // Použijeme ukazatel na číslo řádky
            move_str: line.to_string(), // Předpokládáme, že řádek reprezentuje tah
        };
        let new_move = Move::OkMove(move_info);


        // if let Some(sequence) = &ctx.current_sequence {
        //     let mut sequence = sequence.borrow_mut();
        //     sequence.moves.moves.push(new_move.clone());
        // }

        return new_move
    } else {
        return Move::NoMove("".to_string());
        //panic!("Could not parse number from line {}", line)
    }

}

fn create_context(ctx: &mut ParserContext, line: &str)  {
    //let main_sequence = create_sequence(Vec::new(), Weak::new(), 1);

    ctx.line_num_from = 1;
    ctx.line_num_to = 4;
    ctx.is_initialized = true;

    let uuid = Uuid::new_v4();

    ctx.current_sequence = uuid; //Some(main_sequence.clone());
    ctx.main_sequence = uuid;
}

fn main() -> io::Result<()> {
    // Replace "your_file.txt" with the actual path to your Shift-JIS file.
    let filename = "/Users/marek/RustroverProjects/yasai2/shift-jis-reader/sample.kif";

    let root = Uuid::new_v4();

    // let mut context: ParserContext = ParserContext {
    //     line_num_from: 0,
    //     line_num_to: 0,
    //     move_from: 0,
    //     move_to: 0,
    //
    //     seqs: Default::default(),
    //     is_initialized: false,
    //     context_name: String::from("main"),
    //     current_sequence: uuid,
    //     main_sequence: uuid,
    // };


    let mut context = ParserContext::new("main");
    context.create_sequence(1 );

    println!("Reading file: {}", filename);

    let start = Instant::now();

    match read_shift_jis_lines(filename) {
        Ok(lines) => {

            let mut parser_state = ParserState::Initial;
            for (line_num, line) in lines.iter().enumerate() {
                //println!("{}", line);

                match parser_state {
                    ParserState::Initial => {
                        if line.starts_with("手数") {
                            parser_state = ParserState::MainLine;
                            println!("Main line at line {}", line_num + 1);
                        }

                        if line.starts_with("変化") {
                            parser_state = ParserState::InVariation1;
                            //println!("Invariation line at line {}", line_num + 1);
                        }
                    }
                    ParserState::MainLine => {
                        if line.is_empty() {
                            parser_state = ParserState::Initial;
                            //println!("back to initial at line {}", line_num+1);
                        } else {
                            let num = parse_number_from_line(line, &context);
                            context.add_move(num);
                            // match num {
                            //     Move::OkMove(m) => {
                            //         context.add_move_to_current_sequence(m.line_num, &m.move_str);
                            //     }
                            //     Move::NoMove(_) => {}
                            // }
                            //context.add_move_to_current_sequence()

                        }

                        // parse line
                    }

                    ParserState::InVariation1 => {
                        parser_state = ParserState::InVariation;

                        let mut num = parse_number_from_line(line, &context);
                        match num {
                            Move::OkMove(ref mut m) => {
                                context.add_variation(m.line_num as usize);
                                context.add_move(num);
                                //create_variation(&mut context, m.line_num as usize, Vec::new(), m.move_str.clone());
                                //context.add_move_to_current_sequence(m.line_num, &m.move_str);
                            }
                            Move::NoMove(_) => {}
                        }
                    }

                    ParserState::InVariation => {
                        if line.is_empty() {
                            parser_state = ParserState::Initial;
                            //println!("back from variation at line {}", line_num+1);

                            // print out current variation
                        } else {
                            let num = parse_number_from_line(line, &context);
                            match num {
                                Move::OkMove(ref m) => {
                                    //context.add_move_to_current_sequence(m.line_num, &m.move_str);
                                    context.add_move(num);
                                }
                                Move::NoMove(_) => {}
                            }
                        }

                        //println!(" new variation at line {}", line_num+1);

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

    context.dump_sequences(String::from("main"));
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

            seqs: Default::default(),
            is_initialized: false,
            context_name: String::from("main"),
            current_sequence: uuid,
            main_sequence: uuid,
        };

        //create_context(&mut context, "1234567890");
        context.create_sequence(1);

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

        // if let Some(current_sequence) = &context.main_sequence {
        //     println!("Displaying ShogiSequence context and follow-ups:");
        //     display_context_details(current_sequence, 1);
        // } else {
        //     println!("No current sequence to display.");
        // }


        //display_context_details(context.main_sequence, 1);

        //create_variation(&mut context, 3, Vec::new(), String::from("M3v"));

        context.add_variation(3);

        context.add_move(Move::OkMove(MoveInfo {
            line_num: 3,
            move_str: "M3v".to_string(),
        }));

        context.dump_sequences(String::from("after v1 from move 3"));

        //context.display_context_details();
        // if let Some(current_sequence) = &context.main_sequence {
        //     println!("Displaying ShogiSequence context and follow-ups:");
        //     display_context_details(current_sequence, 1);
        // } else {
        //     println!("No current sequence to display.");
        // }

        //create_variation(&mut context, 2, Vec::new(), String::from("M2vv"));
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

        //context.display_context_details();
        // if let Some(current_sequence) = &context.main_sequence {
        //     println!("Displaying ShogiSequence context and follow-ups:");
        //     display_context_details(current_sequence, 1);
        // } else {
        //     println!("No current sequence to display.");
        // }
    }
}