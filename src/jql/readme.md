## Grammar

```bnf
<pipeline> ::= <expression>
             | <expression> <space> "|" <space> <pipeline>

<expression> ::= <access>
               | <filter>

<access> ::= <root_access> 
           | <access_step>+

<root_access> ::= "."

<access_step> ::= "." <string>
                | "." "[" <number> "]"

<filter> ::= "filter" "(" <condition> ")"

<condition> ::= <comparision>
              | "("+ <condition> ")"+ <space> <logical_op> <space> <comparision>

<logical_op> ::= "and" | "or"

<comparision> ::= <operand> <space> <com_op> <space> <operand>

<operand> ::= <access> | <literal>

<com_op> ::= "==" | ">=" | "<=" | ">" | "<" | "!="

<literal> ::= <string_value> | <boolean> | <number>

<number> ::= [0-9]

<boolean> ::= "true" | "false"

<string> ::= [a-z] | [A-Z]

<null> ::= "null"

<space> ::= " "

<string_value> ::= "\"" ([a-z] | [A-Z] | [0-9] | " ") "\""
```

## Examples
```
.[1] | filter(.id > 10 && .age < 20 && (.money > 10 || .gold >= 1))
```

## Parser

```
Pipeline
├── Access
│   └── Index(1)
│
└── Call("filter")
    │
    └── And
        ├── And
        │   ├── GreaterThan
        │   │   ├── Access("id")
        │   │   └── 10
        │   │
        │   └── LessThan
        │       ├── Access("age")
        │       └── 20
        │
        └── Or
            ├── GreaterThan
            │   ├── Access("money")
            │   └── 10
            │
            └── GreaterThanOrEqual
                ├── Access("gold")
                └── 1
```
