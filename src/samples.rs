//! Short built-in books (original sample text) so the shelf isn't empty.

use crate::epub::{Book, Chapter, TocEntry};
use crate::html::{Block, Kind, Span};
use std::collections::HashMap;

fn span(t: &str) -> Span {
    Span { text: t.to_string(), bold: false, italic: false }
}

fn para(t: &str) -> Block {
    // *word* marks italic, **word** marks bold
    let mut spans = Vec::new();
    let mut bold = false;
    let mut italic = false;
    let mut cur = String::new();
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '*' {
            if !cur.is_empty() {
                spans.push(Span { text: std::mem::take(&mut cur), bold, italic });
            }
            if chars.peek() == Some(&'*') {
                chars.next();
                bold = !bold;
            } else {
                italic = !italic;
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        spans.push(Span { text: cur, bold, italic });
    }
    Block { kind: Kind::Para, spans }
}

fn chapter(title: &str, paras: &[&str]) -> Chapter {
    let mut blocks = vec![Block { kind: Kind::Heading(1), spans: vec![span(title)] }];
    for p in paras {
        if let Some(item) = p.strip_prefix("- ") {
            let mut b = para(item);
            b.kind = Kind::Item;
            blocks.push(b);
        } else if *p == "***" {
            blocks.push(Block { kind: Kind::Rule, spans: vec![] });
        } else {
            blocks.push(para(p));
        }
    }
    Chapter::new(title, blocks)
}

pub struct Sample {
    pub title: &'static str,
    pub author: &'static str,
}

pub const SAMPLES: [Sample; 4] = [
    Sample { title: "Welcome to Folio", author: "Folio Guide" },
    Sample { title: "The Lighthouse at Gull Point", author: "Mara Ashdown" },
    Sample { title: "Small Gardens", author: "Ren Fennimore" },
    Sample { title: "A Year of Bread", author: "Iris Calder" },
];

pub fn book(i: usize) -> Book {
    let chapters = match i {
        0 => vec![
            chapter("Your library", &[
                "Folio keeps all of your e-books on one shelf. Every book shows its cover, its author and how far you have read, so picking up where you left off takes a single click.",
                "To add books, use the **Add books** button at the top of the library. You can pick single EPUB files or point Folio at a whole folder, and it will find every book inside. You can also simply drag EPUB files onto the window.",
                "The tabs along the top split your shelf into books you are *reading*, books you have not started, books you have *finished* and your *favourites*. Search by title or author, and sort the shelf by what you read last, title, author, progress or the date you added a book.",
                "- Right-click a book to mark it as a favourite, mark it finished or take it off the shelf.",
                "- Switch between a grid of covers and a compact list with the view buttons.",
                "- Removing a book from the shelf never deletes the file on your computer.",
            ]),
            chapter("Reading", &[
                "Click a book to open it. Folio remembers the chapter and the exact spot you reached, and brings you back there next time.",
                "Press the right arrow, Page Down or the space bar to move forward one screen, and the left arrow or Page Up to go back. When you reach the end of a chapter the next one opens. You can also scroll with the mouse wheel as usual.",
                "**Contents** lists every chapter of the book. **Bookmarks** keeps the places you have saved: press the bookmark button in the top bar to save the place you are reading, and click a saved bookmark to jump back to it.",
                "At the bottom of the page you can see which chapter you are in, how much of the whole book you have read and roughly how many minutes are left in the chapter.",
            ]),
            chapter("Make it yours", &[
                "Press **Aa** while reading to change how the page looks. Choose one of four themes: a bright paper page, warm sepia, a soft dark page for the evening and true black for reading in the dark.",
                "Make the text larger or smaller, pick a serif or a sans-serif typeface, give the lines more room and choose how wide the column of text should be. You can also justify the text and indent the first line of each paragraph, like a printed book.",
                "Your choices are saved and used for every book.",
                "***",
                "That's all there is to it. Enjoy your reading.",
            ]),
        ],
        1 => vec![
            chapter("The keeper", &[
                "The lighthouse at Gull Point had been painted white so many times that its walls seemed soft, like a candle left too near the stove. Every spring the keeper climbed a ladder with a bucket and added one more coat, and every winter the sea took a little of it back.",
                "Annet had kept the light for eleven years. She knew the sound of each step on the iron stair, the third one that rang higher than the rest and the twentieth that groaned as if it remembered an old injury. At dusk she climbed them all, trimmed the wick, wiped the great lens until it shone, and waited for the dark to come in from the water.",
                "Ships rarely passed now. The fishing boats went out from the harbour on the far side of the headland, and the big freighters kept to the deep channel miles away. Still, the light turned every night, and Annet wrote in the log every morning: *Clear. Light kept. No vessels sighted.*",
                "She did not mind. There was a kind of company in the turning beam, in the way it touched the rocks and the gulls and the slow grey backs of the waves, one after another, as if counting them.",
            ]),
            chapter("The storm", &[
                "The storm came on a Tuesday, which Annet would remember later because Tuesday was the day the supply boat should have come and did not.",
                "By noon the sky had turned the colour of an old bruise. By three the rain was moving sideways across the windows and the whole tower hummed in the wind like a struck bell. She lit the lamp early and stayed beside it, feeding it, watching the glass for cracks.",
                "A little after nine she saw a light where no light should be: a small, frightened yellow point low on the water, rising and falling far too close to the reef.",
                "She did the only thing a keeper can do. She kept the beam turning, steady and bright, so that whoever was out there would know exactly where the rocks were, and where they were not.",
                "For an hour the small light wandered. Then, slowly, it began to move away from the reef, around the point, towards the calm water of the harbour.",
            ]),
            chapter("Morning", &[
                "In the morning the sea was flat and tired. A boy from the harbour climbed the path with a basket of bread and a jar of honey and a message from his uncle, who had been out in a small boat the night before and had followed the light home.",
                "Annet thanked him and gave him tea, and after he had gone she opened the log and wrote, a little more carefully than usual: *Storm. Light kept. One vessel sighted, and safe.*",
                "Then she went out with her bucket and her ladder, because it was spring again, and the walls needed another coat.",
            ]),
        ],
        2 => vec![
            chapter("Starting with a pot", &[
                "A garden does not need a field. It can begin with a single pot on a windowsill, a handful of soil and something that wants to grow.",
                "Start with plants that forgive you: mint, chives, a geranium, a small tomato. They will tell you clearly when they are thirsty or unhappy, and they recover quickly when you get it right.",
                "Choose a pot with a hole in the bottom. More small plants are lost to standing water than to any pest, and a saucer beneath the pot is all you need to keep the sill dry.",
                "- Fill the pot with fresh potting mix, not soil from the yard.",
                "- Water until it runs out of the bottom, then wait until the top feels dry.",
                "- Turn the pot a quarter turn every few days so the plant grows straight.",
            ]),
            chapter("Light and water", &[
                "Most of what a small garden needs comes down to two things: how much light it gets and how often it is watered.",
                "Watch your window for a whole day. A sill that is bright from morning to evening suits herbs and tomatoes; one that only sees the sun for an hour or two is better for ferns and leafy plants.",
                "Water in the morning when you can. The plant has the whole day to drink, and the leaves dry before the cool of the night.",
                "Above all, look at your plants every day. A garden that is noticed is almost always a garden that does well.",
            ]),
        ],
        _ => vec![
            chapter("Flour, water, salt", &[
                "Bread is one of the oldest things people make, and one of the simplest. Flour, water and salt, a little yeast and a little time: that is all it asks.",
                "The first loaf you bake will probably be dense and slightly lopsided, and it will still be one of the best things you have ever eaten, because you made it.",
                "Mix the flour and water first and let them rest for half an hour before you add anything else. The flour drinks the water, and the dough becomes smooth almost by itself.",
            ]),
            chapter("Patience", &[
                "The secret ingredient in good bread is patience. Dough that rises slowly, in a cool kitchen or overnight in the refrigerator, tastes deeper and keeps longer.",
                "Learn to read the dough rather than the clock. When it has grown by half and springs back slowly when you press it with a floured finger, it is ready.",
                "Bake it hotter than you think, and longer than you think. A deep brown crust is flavour, not failure.",
                "And then, hardest of all, let it cool before you cut it.",
            ]),
        ],
    };
    let toc = chapters
        .iter()
        .enumerate()
        .map(|(i, c)| TocEntry { label: c.title.clone(), chapter: i, depth: 0 })
        .collect();
    let s = &SAMPLES[i.min(SAMPLES.len() - 1)];
    Book {
        title: s.title.to_string(),
        author: s.author.to_string(),
        chapters,
        toc,
        images: HashMap::new(),
        cover: None,
    }
}
