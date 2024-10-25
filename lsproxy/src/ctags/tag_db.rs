use crate::api_types::{FilePosition, FileRange, Position, Symbol};
use polars::prelude::*;

#[derive(Debug)]
pub struct TagDatabase {
    df: DataFrame,
}

impl TagDatabase {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let df = DataFrame::new(vec![
            Series::new("name", Vec::<String>::new()),
            Series::new("kind", Vec::<String>::new()),
            Series::new("language", Vec::<String>::new()),
            Series::new("file_name", Vec::<String>::new()),
            Series::new("start_line", Vec::<u32>::new()),
            Series::new("start_character", Vec::<u32>::new()),
            Series::new("end_line", Vec::<u32>::new()),
        ])?;
        Ok(Self { df })
    }

    pub fn add_tags_by_columns(
        &mut self,
        names: Vec<String>,
        kinds: Vec<String>,
        languages: Vec<String>,
        files: Vec<String>,
        start_lines: Vec<u32>,
        start_characters: Vec<u32>,
        end_lines: Vec<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let new_df = DataFrame::new(vec![
            Series::new("name", names),
            Series::new("kind", kinds),
            Series::new("language", languages),
            Series::new("file_name", files),
            Series::new("start_line", start_lines),
            Series::new("start_character", start_characters),
            Series::new("end_line", end_lines),
        ])?;

        self.df = match &self.df.height() {
            0 => new_df,
            _ => self.df.vstack(&new_df)?,
        };

        Ok(())
    }

    pub fn clear(&mut self) {
        self.df.clear();
    }

    pub fn get_file_symbols(
        &self,
        file_name: &str,
    ) -> Result<Vec<Symbol>, Box<dyn std::error::Error>> {
        let filtered_df = self
            .df
            .clone()
            .lazy()
            .filter(col("file_name").eq(lit(file_name)))
            .sort_by_exprs(
                vec![col("start_line"), col("start_character")],
                vec![false, false],
                false,
                false,
            )
            .collect()?;

        let names = filtered_df.column("name")?.str()?;
        let kinds = filtered_df.column("kind")?.str()?;
        let files = filtered_df.column("file_name")?.str()?;
        let start_lines = filtered_df.column("start_line")?.u32()?;
        let start_characters = filtered_df.column("start_character")?.u32()?;
        let end_lines = filtered_df.column("end_line")?.u32()?;

        let mut results = Vec::with_capacity(filtered_df.height());
        for i in 0..filtered_df.height() {
            results.push(Symbol {
                name: names.get(i).expect("Row index out of bounds").to_string(),
                kind: kinds.get(i).expect("Row index out of bounds").to_string(),
                identifier_position: FilePosition {
                    path: files.get(i).expect("Row index out of bounds").to_string(),
                    position: Position {
                        line: start_lines.get(i).expect("Row index out of bounds"),
                        character: start_characters.get(i).expect("Row index out of bounds"),
                    },
                },
                range: FileRange {
                    path: files.get(i).expect("Row index out of bounds").to_string(),
                    start: Position {
                        line: start_lines.get(i).expect("Row index out of bounds"),
                        character: 0,
                    },
                    end: Position {
                        line: end_lines.get(i).expect("Row index out of bounds"),
                        character: 0,
                    },
                },
            });
        }
        Ok(results)
    }
}
