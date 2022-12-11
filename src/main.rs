#![allow(unused)]
use regex::{Error, Regex};
use std::borrow::{Borrow, Cow};
use std::fs::{self, write, DirEntry, FileType, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{stdin, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::str::Chars;
use std::{clone, default};
use std::{collections::HashMap, fmt::format, ops::Index, sync::Arc};
use std::{fs::File, io::Read};
use walkdir::WalkDir;

use joinery::JoinableIterator;
#[derive(Debug)]
struct Constructor<'a> {
    name: Option<&'a str>,
    arguments: Vec<Argument<'a>>,
    seald_class: String,
}
#[derive(Debug)]
struct Argument<'a> {
    name: &'a str,
    typo: &'a str,
    default: Option<&'a str>,
    nullable: bool,
    optional: bool,
    named: bool,
}
impl<'a> Eq for Argument<'a> {}
impl<'a> Hash for Argument<'a> {
    fn hash<S: std::hash::Hasher>(&self, state: &mut S) {
        self.name.hash(state);
        self.typo.hash(state);
    }
}
impl<'a> PartialEq for Argument<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.typo == other.typo
    }
}
fn to_path_clone(path: &Path) -> Vec<PathBuf> {
    vec![path.parent().unwrap().to_path_buf()]
}
fn main() {
    let exe = std::env::current_exe().unwrap();
    let root = exe.parent().unwrap().to_str().expect("Error :(");

    // for entry in WalkDir::new(root)
    //     .into_iter()
    //     .filter_entry(|entry| {
    //         true // entry.file_type().is_file()  //&& entry.file_name().to_str().unwrap().ends_with(".dart")
    //     })
    //     .filter_map(|f| f.ok())
    // {
    //     if entry.file_name().to_str().unwrap().ends_with(".dart") {
    //         println!("{:?}", entry.file_type());
    //     }
    // }

    // let dart_files: Vec<walkdir::DirEntry> = WalkDir::new(root)
    //     .into_iter()
    //     .filter_entry(|entry| entry.file_name().to_str().unwrap().ends_with(".dart"))
    //     .filter_map(|f| f.ok())
    //     .collect();
    // // .inspect(|f| println!("{f:?}"));
    // println!("{dart_files:?}");

    // let walk_dir = WalkDirGeneric::<((usize), (bool))>::new(root).max_depth(6).min_depth(3).skip_hidden(false).follow_links(true)
    //     .process_read_dir(|depth, path, read_dir_state, children| {
    //         // 2. Custom filter
    //         children.retain(|dir_entry_result| {
    //             println!("{:?}",dir_entry_result);
    //             dir_entry_result
    //                 .as_ref()
    //                 .map(|dir_entry| {
    //                     dir_entry
    //                         .file_name
    //                         .to_str()
    //                         .map(|s| s.ends_with(".dart"))
    //                         .unwrap_or(false)
    //                 })
    //                 .unwrap_or(false)
    //         });
    //     });

    // let dart_files: Vec<jwalk::DirEntry<(usize, bool)>> =
    //     walk_dir.into_iter().filter_map(|f| f.ok()).collect();

    // let dart_files = fts::walkdir::WalkDir::new(WalkDirConf::new(path).no_metadata());

    let mut parts: Vec<(PathBuf, String)> = Vec::new();
    for file in WalkDir::new(root).into_iter().filter_map(|f| match f {
        Ok(entry)
            if entry.file_name().to_str().is_some()
                && entry.file_name().to_str().unwrap().ends_with(".dart") =>
        {
            Some(entry)
        }
        _ => None,
    }) {
        let re = Regex::new(r"//.*").unwrap();
        let data = fs::read_to_string(file.path()).ok();
        if data == None {
            continue;
        }
        let data = data.unwrap();
        let data = re.replace_all(data.as_str(), "");
        if data.contains("@freezed") {
            let re = Regex::new(r#"part of ("|').*("|');"#).unwrap();
            let part = re
                .find(data.borrow())
                .map(|f| &f.as_str()[9..f.as_str().len() - 2]);
            if let Some(part) = part {
                if parts.contains(&(
                    file.path().parent().expect("parent error").to_path_buf(),
                    part.to_string(),
                )) {
                    let res = generate_code(data.borrow());

                    let file = OpenOptions::new()
                        .append(true)
                        .open(
                            file.path()
                                .parent()
                                .unwrap()
                                .join(part)
                                .with_extension("freezed.dart")
                                .to_str()
                                .unwrap(),
                        )
                        .unwrap();
                    if let Some(res) = res {
                        let mut f = BufWriter::new(file);
                        write!(f, "{res}");
                    }
                } else {
                    let res = generate_code(data.borrow());

                    if let Some(mut res) = res {
                        res.insert_str(0, format!("part of '{part}';\n").as_str());

                        fs::write(
                            file.path()
                                .parent()
                                .unwrap()
                                .join(part)
                                .with_extension("freezed.dart")
                                .to_str()
                                .unwrap(),
                            res,
                        );
                        println!("{}", file.path().to_str().unwrap());

                        parts.push((
                            file.path().parent().unwrap().to_path_buf(),
                            part.to_string(),
                        ));
                    }
                }
            } else {
                let res = generate_code(data.borrow());

                if let Some(mut res) = res {
                    res.insert_str(
                        0,
                        format!("part of '{}';\n", file.file_name().to_str().unwrap()).as_str(),
                    );

                    fs::write(
                        file.path().with_extension("freezed.dart").to_str().unwrap(),
                        res,
                    );
                    println!("{}", file.path().to_str().unwrap());
                }
            }
        }
    }
}

fn generate_code(s: &str) -> Option<String> {
    let re = Regex::new(r"class[\s]*(\w)*[\s]*with[\s]*_\$(\w)*").unwrap();
    let abstract_builder = re.find(s.borrow())?.as_str();
    let abstract_builder: Vec<&str> = abstract_builder.split_whitespace().collect();
    let abstract_class = abstract_builder.get(1)?.to_string();
    let abstract_class_mixin = abstract_builder.get(3)?.to_string();

    let mut constructors: Vec<Constructor> = vec![];
    let main_scope = find_scope(
        s.borrow(),
        s.match_indices("@freezed").next().map(|f| f.0)?,
        '{',
        '}',
    )
    .unwrap();

    let splited = main_scope.split(';').filter(|&a| a.contains("factory"));
    for item in splited {
        // println!("{item}");
        let constructor_name = Regex::new(r"(\.)([\w]*)")
            .unwrap()
            .find(item)
            .map(|f| f.as_str().trim_start_matches('.'));
        let seald_class = item.rsplit_once('=').unwrap().1.trim().to_string();
        let par_scope = find_scope(
            item,
            item.match_indices("factory").next().unwrap().0,
            '(',
            ')',
        )
        .unwrap();
        let args = separate_args(par_scope, ',');
        // println!("{args:?}");

        let mut arguments: Vec<Argument> = vec![];
        arguments = extract_arguments(args, arguments, '(', false);

        constructors.push(Constructor {
            seald_class,
            name: constructor_name,
            arguments,
        });
    }
    if (constructors.iter().filter(|&f| f.name.is_none())).count() > 1 {
        println!("{} : The unnamed constructor is already defined. Try giving one of the constructors a name.",abstract_class);
        return None; //TODO: Better Error Handling
    }

    let constructor_length = constructors.len();
    let mut args_map: HashMap<&Argument, i32> = HashMap::new();
    for c in constructors.iter() {
        for e in c.arguments.iter() {
            if args_map.contains_key(e)
            //TODO: It's probably not working due to Eq and Hash
            {
                let new_val = args_map[&e] + 1;
                args_map.insert(e, new_val);
            } else {
                args_map.insert(e, 1);
            }
        }
    }
    // println!("{args_map:?}");
    // println!("{constructor_length}");
    let abstract_arguments: Vec<&Argument> = args_map
        .iter()
        .filter(|&e| (*e.1 as usize) == constructor_length)
        .map(|e| *e.0)
        .collect();

    let gets = abstract_arguments
        .iter()
        .map(|e| {
            format!(
                "{} get {} => throw  UnimplementedError();\n",
                e.typo, e.name
            )
        })
        .join_with("");

    let funcs = constructors
        .iter()
        .filter_map(|c| {
            c.name
                .map(|c_name| format!("required T Function({} value) {},\n", c.seald_class, c_name))
        })
        .join_with("")
        .to_string();

    let nullable_funcs = constructors
        .iter()
        .filter_map(|c| {
            c.name
                .map(|c_name| format!("T Function({} value)? {},\n", c.seald_class, c_name))
        })
        .join_with("")
        .to_string();

    let default_func = constructors
        .iter()
        .filter(|a| a.name.is_none())
        .next()
        .map_or("".to_string(), |d| {
            format!("T Function({} value) $default,", d.seald_class)
        });

    let default_func_nullable = constructors
        .iter()
        .filter(|a| a.name.is_none())
        .next()
        .map_or("".to_string(), |d| {
            format!("T Function({} value)? $default,", d.seald_class)
        });

    let sealed_classes = constructors
        .iter()
        .map(
            |Constructor {
                 name,
                 arguments,
                 seald_class,
             }| {
                let args = arguments
                    .iter()
                    .map(|arg| {
                        let required = if arg.named && arg.optional == false {
                            "required "
                        } else {
                            ""
                        };
                        let default = if let Some(d) = arg.default.as_ref() {
                            format!(" = {d}")
                        } else {
                            "".to_string()
                        };
                        format!("{required}this.{}{default}", arg.name)
                    })
                    .join_with(',')
                    .to_string();
                // if !abstract_arguments.is_empty() {}

                let overrides = arguments
                    .iter()
                    .map(|Argument { name, typo, .. }| {
                        let mut ov_signe = "";
                        if abstract_arguments
                            .iter()
                            .any(|e| e.name == *name && e.typo == *typo)
                        {
                            ov_signe = "@override";
                        }
                        format!("{ov_signe}\nfinal {typo} {name}\n;")
                    })
                    .join_with('\n')
                    .to_string();

                let copy_with_args = arguments
                    .iter()
                    .map(
                        |Argument {
                             name,
                             typo,
                             nullable,
                             ..
                         }| {
                            format!("{typo}{q} {name}", q = if *nullable { "" } else { "?" })
                            //Yes it seems opposite but it's correct look wisely!
                        },
                    )
                    .join_with(',')
                    .to_string();
                let copy_with_constr = arguments
                    .iter()
                    .map(|Argument { name, .. }| format!("{name}: {name}?? this.{name}"))
                    .join_with(',')
                    .to_string();
                let ove_sign = match abstract_arguments.len() {
                    0 => "",
                    _ => "@override",
                };
                let mixin_methods_imple = if funcs.is_empty() {
                    if default_func.is_empty() || copy_with_args.is_empty() {
                        format!("")
                    } else {
                        format!(
                            "{ove_sign}
                            {abstract_class} copyWith({{{copy_with_args}}}) {{
                                return {abstract_class}({copy_with_constr});
                            }}"
                        )
                    }
                } else {
                    let constr = if let Some(name) = name {
                        format!(".{name}")
                    } else {
                        "".to_string()
                    };
                    if (copy_with_args.is_empty()) {
                        format!(
                            "@override
T map<T extends Object?>({default_func}{{
{funcs}
}}) =>{name}(this);
@override
T? mapOrNull<T extends Object?>({default_func_nullable}{{
{nullable_funcs}
}}) =>{name}?.call(this);
@override
T maybeMap<T extends Object?>({default_func_nullable}{{
{nullable_funcs}
required T Function() orElse,
}}) =>{name}?.call(this) ?? orElse();",
                            name = name.unwrap_or("$default")
                        )
                    } else {
                        format!(
                            "@override
T map<T extends Object?>({default_func}{{
{funcs}
}}) =>{name}(this);
 @override
 T? mapOrNull<T extends Object?>({default_func_nullable}{{
 {nullable_funcs}
 }}) =>{name}?.call(this);
 @override
 T maybeMap<T extends Object?>({default_func_nullable}{{
 {nullable_funcs}
required T Function() orElse,
 }}) =>{name}?.call(this) ?? orElse();

{ove_sign}
{abstract_class} copyWith({{{copy_with_args}}}) {{
    return {abstract_class}{constr}({copy_with_constr});
}}",
                            name = name.unwrap_or("$default")
                        )
                    }
                };
                if args.is_empty() {
                    format!(
                        "class {seald_class} implements {abstract_class} {{
const {seald_class}();
{overrides}
{mixin_methods_imple}}}"
                    )
                } else {
                    format!(
                        "class {seald_class} implements {abstract_class} {{
const {seald_class}({{{args}}});
{overrides}
{mixin_methods_imple}}}"
                    )
                }
            },
        )
        .join_with('\n')
        .to_string();

    let copy_with_args = abstract_arguments
        .iter()
        .map(
            |Argument {
                 name,
                 typo,
                 nullable,
                 ..
             }| format!("{typo}{q} {name}", q = if *nullable { "" } else { "?" }), //Yes it seems opposite but it's correct. Look wisely!
        )
        .join_with(',')
        .to_string();

    let copy_with = match abstract_arguments.len() {
        0 => format!(""),
        _ => format!(
            "{abstract_class} copyWith({{{copy_with_args}}}) => throw UnimplementedError();"
        ),
    };
    if (funcs.is_empty()) {
        Some(format!(
            "mixin {abstract_class_mixin} {{
            {gets}
            {copy_with}

            }}

           {sealed_classes}
            "
        ))
    } else {
        Some(format!(
            "mixin {abstract_class_mixin} {{\n{gets}\nT map<T extends Object?>({default_func}{{\n{funcs}}}) => throw UnimplementedError();\nT? mapOrNull<T extends Object?>({default_func}{{{nullable_funcs}}}) => throw UnimplementedError();\nT maybeMap<T extends Object?>({default_func}{{{nullable_funcs}\nrequired T Function() orElse,}}) => throw UnimplementedError();{copy_with}}}\n{sealed_classes}"))
    }
}

fn find_scope(source: &str, start_index: usize, start_char: char, end_char: char) -> Option<&str> {
    let mut first = (&source[start_index..])
        .match_indices(start_char)
        .next()
        .unwrap()
        .0
        + start_index;

    let chars = (source[first..]).chars();
    let mut counter = 0;
    let last = || {
        for char in chars.enumerate() {
            if char.1 == start_char {
                counter += 1;
            } else if char.1 == end_char {
                counter -= 1;
            }
            if counter == 0 {
                return char.0;
            }
        }
        return 0;
    };
    let last = first + last();
    Some(&source[first + 1..last]) // +1 is very important!
}

fn separate_args<'a>(source: &'a str, delimiter: char) -> Vec<&'a str> {
    let mut counter = 0;
    let mut list: Vec<&'a str> = vec![];

    let (
        mut squar_brac_counter,
        mut angel_brac_counter,
        mut qurly_brac_counter,
        mut round_brac_counter,
    ) = (0, 0, 0, 0);
    let mut is_single_quot = false;
    let mut is_double_quot = false;
    let mut ex_char_was_back_slash = false;
    for char in source.chars().enumerate() {
        if char.1 == '\'' && !is_double_quot && !ex_char_was_back_slash {
            is_single_quot = !is_single_quot;
        }
        if char.1 == '"' && !is_single_quot && !ex_char_was_back_slash {
            is_double_quot = !is_double_quot;
        }
        if char.1 == '(' && !is_double_quot && !is_single_quot {
            round_brac_counter += 1;
        }
        if char.1 == ')' && !is_double_quot && !is_single_quot {
            round_brac_counter -= 1;
        }
        if char.1 == '<' && !is_double_quot && !is_single_quot {
            angel_brac_counter += 1;
        }
        if char.1 == '>' && !is_double_quot && !is_single_quot {
            angel_brac_counter -= 1;
        }
        if char.1 == '[' && !is_double_quot && !is_single_quot {
            squar_brac_counter += 1;
        }
        if char.1 == ']' && !is_double_quot && !is_single_quot {
            squar_brac_counter -= 1;
        }
        if char.1 == '{' && !is_double_quot && !is_single_quot {
            qurly_brac_counter += 1;
        }
        if char.1 == '}' && !is_double_quot && !is_single_quot {
            qurly_brac_counter -= 1;
        }
        if char.1 == delimiter
            && !is_double_quot
            && !is_single_quot
            && (
                squar_brac_counter,
                angel_brac_counter,
                qurly_brac_counter,
                round_brac_counter,
            ) == (0, 0, 0, 0)
        {
            list.push(&source[counter..char.0]);
            counter = char.0 + 1;
        }
        ex_char_was_back_slash = char.1 == '\\';
    }
    list.push(&source[counter..]);
    list
}

fn extract_arguments<'a>(
    args: Vec<&'a str>,
    mut arguments: Vec<Argument<'a>>,
    scope_char: char,
    named: bool,
) -> Vec<Argument<'a>> {
    for &arg in args.iter() {
        let arg = arg.trim();
        if arg.starts_with('{') {
            let inner_args = separate_args(&arg[1..arg.len() - 1], ',');
            arguments = extract_arguments(inner_args, arguments, '{', true);
        } else if arg.starts_with('[') {
            let inner_args = separate_args(&arg[1..arg.len() - 1], ',');
            arguments = extract_arguments(inner_args, arguments, '[', false);
        } else if !arg.is_empty() {
            let mut optional = false;
            if scope_char == '[' || scope_char == '{' {
                optional = true;
            }
            let mut items = separate_args(arg, ' ');
            let mut default = None;
            let &name = items.last().unwrap();
            let typo = items[items.len() - 2];
            for &item in items.iter() {
                if item == "required" {
                    optional = false;
                }
                if item.starts_with("@Default") {
                    default = find_scope(item, 7, '(', ')');
                    optional = true;
                }
            }
            arguments.push(Argument {
                name,
                typo,
                default,
                nullable: typo.ends_with("?"),
                optional,
                named,
            });
        }
    }
    arguments
}

#[cfg(test)]
mod tests {
    use super::*;
    // #[test]
    fn test_separate_args() {
        let res = separate_args(r#"sdf,[string boldi="salam"],"#, ',');
        println!("{:?}", res);
    }

    // #[test]
    fn find_scop_test() {
        let scop = find_scope(
            "
class Person with _$Person {
  const factory Person({
    String? name,
    int? age,
    Gender? gender,
  }
  ) = _Person;
  const factory Person.kom({String? name, Gender? gender}) = _Kom;
}",
            0,
            '{',
            '}',
        )
        .unwrap();
        println!("{scop}");
    }

    // #[test]
    fn walk_dir() {
        let r: Vec<DirEntry> = fs::read_dir("archive")
            .unwrap()
            .filter_map(|f| f.ok())
            .collect();
        println!("{:?}", r);
        let dart_files = walkdir::WalkDir::new("archive")
            .into_iter()
            .filter_map(|file| match file {
                Ok(file)
                    if file.metadata().unwrap().is_file() && file.path().ends_with(".dart") =>
                {
                    Some(file)
                }
                Err(er) => {
                    println!("{er:?}");
                    None
                }
                _ => None,
            })
            .inspect(|f| println!("{f:?}"));
        println!("saalm");
    }

    #[test]
    fn argument_eq_test() {
        let a1 = Argument {
            default: None,
            name: "asd",
            typo: "wewewe",
            named: false,
            nullable: true,
            optional: true,
        };
        let a2 = Argument {
            default: Some("asd"),
            name: "asd",
            typo: "wewewe",
            named: true,
            nullable: false,
            optional: false,
        };
        let mut map: HashMap<&Argument, i32> = HashMap::new();
        map.insert(&a1, 0);

        assert!(map.contains_key(&a2));
    }
}
