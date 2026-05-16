use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "qurts.pest"]
pub struct QurtsParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let successful_parse = QurtsParser::parse(Rule::program, "fn beans() -> bool {}");
        println!("{:?}", successful_parse);
        let unsuccessful_parse = QurtsParser::parse(Rule::program, "func 1-2 () {}");
        println!("{:?}", unsuccessful_parse);
        assert!(successful_parse.is_ok());
        assert!(unsuccessful_parse.is_err());
    }
}
