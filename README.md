<div align="center">

# wmish

</div>

```plaintext
   __                      _     _          
   \ \ __      ___ __ ___ (_)___| |__       
    \ \\ \ /\ / / '_ ` _ \| / __| '_ \      
    / / \ V  V /| | | | | | \__ \ | | | ____
   /_/   \_/\_/ |_| |_| |_|_|___/_| |_|[____] 
   > A command line tool for WMI _          
```

## About

**wmish** is a command line tool for [WMI (Windows Management Instrumentation](https://learn.microsoft.com/en-us/windows/win32/wmisdk/wmi-start-page) written in Rust.

**WMI** allows easy access to various Windows resources, making it particularly useful for system administrators, security researchers, and engineers who focus on low-level processing.  

**wmish** is a simple tool for accessing WMI from the command line. It supports both interactive mode and script mode, and is convenient for automating system administration tasks and collecting security-related information.

## Features

- **Non-interactive mode**
- **Interactive mode with rich Tab key completions**
- **Script mode**

## Commands

- `wmish run FILE.wmish`: Evaluate script and execute inline commands
- `wmish shell`: Interactive-shell mode that supports Tab-completion
- `wmish query QUERY`: Interactive-shell mode

## Inline Commands

Inline commands are available for `shell` and `run` command.

- [x] `NAMESPACE <Namespace>`: Move around namespaces hieralchy like `cd` command
- [x] `CLASSES`: Show a list of classes on the current namespace like `ls` command
- [ ] `SHOW <ClassName>`: Show properties and methods detailed informations such as names, types, and descriptions, etc... for the specified class name
- [x] `SELECT <Properties...> FROM <ClassName> WHERE <Conditions>...`: Query to WMI in WQL (WMI Query Language)
- [x] `FORMAT CSV|TABLE|JSON`: Set output format (JSON is pretty-printed)
- [ ] `CALL <MethodName> <<PropertyName1=Value1> <PropertyName=Value2> ...> WITH <Query|ClassName>`
- [x] `MOF <ClassName>`: Get MOF (Managed Object Format) of specified class name