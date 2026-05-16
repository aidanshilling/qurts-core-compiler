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
        let successful_parse = QurtsParser::parse(Rule::program, "fn test() -> bool {}");
        println!("{:?}", successful_parse);
        let unsuccessful_parse = QurtsParser::parse(Rule::program, "func 1-2 () {}");
        println!("{:?}", unsuccessful_parse);
        assert!(successful_parse.is_ok());
        assert!(unsuccessful_parse.is_err());
    }

    #[test]
    fn single_lifetime() {
        let result = QurtsParser::parse(Rule::program, "fn test<'a>() -> bool {}");
        assert!(result.is_ok());
    }

    #[test]
    fn constrained_lifetime() {
        let result = QurtsParser::parse(Rule::program, "fn test<'b != '0>() -> qbit {}");
        assert!(result.is_ok());
    }
}
