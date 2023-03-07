use std::cmp::PartialEq;
use std::error::Error;
use std::fmt;
use std::io::ErrorKind;

#[derive(Debug)]
struct CsvDbError(String);

impl fmt::Display for CsvDbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for CsvDbError {}

pub trait CsvRecord {
    fn from_fields(fields: &[String]) -> Self;
    fn to_fields(&self) -> Vec<String>;
}

pub struct Database<'a> {
    path: &'a str,
    extension: &'a str,
}

impl<'a> Database<'a> {
    pub fn new(path: &'a str, extension: Option<&'a str>) -> Self {
        let extension = match extension {
            None => "csv",
            Some(extension) => extension,
        };

        Self { path, extension }
    }

    pub fn select<T, P>(
        &self,
        from: &str,
        where_filter: P,
    ) -> Result<Option<Vec<T>>, Box<dyn Error>>
    where
        T: CsvRecord,
        P: FnMut(&T) -> bool,
    {
        let mut entities: Vec<T> = Vec::new();
        let mut rdr = match csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(format!("{}{}.{}", self.path, from, self.extension))
        {
            Ok(rdr) => rdr,
            Err(error) => match error.kind() {
                csv::ErrorKind::Io(error) => match error.kind() {
                    ErrorKind::NotFound => return Ok(Some(entities)),
                    ErrorKind::PermissionDenied => {
                        return Err(Box::new(CsvDbError("Permission denied".into())))
                    }
                    _ => return Err(Box::new(CsvDbError("Problem reading file".into()))),
                },
                _ => return Err(Box::new(CsvDbError("Unknown problem".into()))),
            },
        };

        for result in rdr.records() {
            let record = result?;
            let mut fields: Vec<String> = Vec::new();

            for field in &record {
                fields.push(String::from(field));
            }

            let entity = T::from_fields(&fields);

            entities.push(entity);
        }

        //if let Some(where_filter) = where_filter {
        entities = entities.into_iter().filter(where_filter).collect();
        //}

        match entities.is_empty() {
            true => Ok(None),
            false => Ok(Some(entities)),
        }
    }

    pub fn insert<T>(&self, into: &str, entity: T) -> Result<(), Box<dyn Error>>
    where
        T: CsvRecord,
    {
        let mut entities: Vec<T> = self.select(into, |_| true)?.unwrap_or_default();

        entities.push(entity);

        self.write(into, &entities.iter().collect())?;

        Ok(())
    }

    pub fn delete<T, P>(&self, from: &str, where_filter: P) -> Result<(), Box<dyn Error>>
    where
        T: CsvRecord + PartialEq,
        P: FnMut(&&T) -> bool,
    {
        let entities = self.select(from, |_| true)?.unwrap_or_default();
        let delete: Vec<&T> = entities.iter().filter(where_filter).collect();
        let mut keep: Vec<&T> = Vec::new();

        for entity in &entities {
            if !delete.contains(&&entity) {
                keep.push(entity)
            }
        }

        self.write(from, &keep)?;

        Ok(())
    }

    pub fn write<T: CsvRecord>(&self, to: &str, entities: &Vec<&T>) -> Result<(), Box<dyn Error>> {
        let mut wtr = csv::WriterBuilder::new()
            .has_headers(false)
            .from_path(format!("{}{}.{}", self.path, to, self.extension))?;

        for entity in entities {
            let fields = entity.to_fields();

            wtr.write_record(fields)?;
        }

        Ok(())
    }
}
