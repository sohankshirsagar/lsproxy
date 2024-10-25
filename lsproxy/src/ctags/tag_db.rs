use crate::api_types::{FilePosition, Position, Symbol};
use polars::prelude::*;

#[derive(Debug)]
pub struct TagDatabase {
    df: DataFrame,
}

impl TagDatabase {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let df = DataFrame::new(vec![
            Series::new("name", Vec::<String>::new()),
            Series::new("file_name", Vec::<String>::new()),
            Series::new("line", Vec::<u32>::new()),
            Series::new("column", Vec::<u32>::new()),
        ])?;
        Ok(Self { df })
    }

    pub fn add_tags_by_columns(
        &mut self,
        names: Vec<String>,
        files: Vec<String>,
        lines: Vec<u32>,
        columns: Vec<u32>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let new_df = DataFrame::new(vec![
            Series::new("name", names),
            Series::new("file_name", files),
            Series::new("line", lines),
            Series::new("column", columns),
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
                vec![col("line"), col("column")],
                vec![false, false],
                false,
                false,
            )
            .collect()?;

        let names = filtered_df.column("name")?.str()?;
        let files = filtered_df.column("file_name")?.str()?;
        let lines = filtered_df.column("line")?.u32()?;
        let columns = filtered_df.column("column")?.u32()?;

        let mut results = Vec::with_capacity(filtered_df.height());
        for i in 0..filtered_df.height() {
            results.push(Symbol {
                name: names.get(i).expect("Row index out of bounds").to_string(),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: files.get(i).expect("Row index out of bounds").to_string(),
                    position: Position {
                        line: lines.get(i).expect("Row index out of bounds"),
                        character: columns.get(i).expect("Row index out of bounds"),
                    },
                },
            });
        }
        Ok(results)
    }

    pub fn find_symbol(&self, name: &str) -> Result<Vec<Symbol>, Box<dyn std::error::Error>> {
        let filtered_df = self
            .df
            .clone()
            .lazy()
            .filter(col("name").eq(lit(name)))
            .sort_by_exprs(
                vec![col("line"), col("column")],
                vec![false, false],
                false,
                false,
            )
            .collect()?;

        let names = filtered_df.column("name")?.str()?;
        let files = filtered_df.column("file_name")?.str()?;
        let lines = filtered_df.column("line")?.u32()?;
        let columns = filtered_df.column("column")?.u32()?;

        let mut results = Vec::with_capacity(filtered_df.height());
        for i in 0..filtered_df.height() {
            results.push(Symbol {
                name: names.get(i).expect("Row index out of bounds").to_string(),
                kind: String::from("ctag_definition"),
                start_position: FilePosition {
                    path: files.get(i).expect("Row index out of bounds").to_string(),
                    position: Position {
                        line: lines.get(i).expect("Row index out of bounds"),
                        character: columns.get(i).expect("Row index out of bounds"),
                    },
                },
            });
        }
        Ok(results)
    }
}
