## Lexical FSM

```
NewToken
--------
{               -> EMIT(LBrace)  ; NewToken
}               -> EMIT(RBrace)  ; NewToken
[               -> EMIT(LBracket); NewToken
]               -> EMIT(RBracket); NewToken
:               -> EMIT(Colon)   ; NewToken
,               -> EMIT(Comma)   ; NewToken
"               -> InString(start = pos+1, end = pos+1)
t               -> InTrue(start = pos, end = pos)
f               -> InFalse(start = pos, end = pos)
n               -> InNull(start = pos, end = pos)
digit | -       -> InNumber(start = pos, end = pos)
<whitespace>    -> NewToken          (skip; do not emit)
_               -> Invalid


InString(span)
--------------
" and prev != \ -> EMIT(String, span.start..pos) ; NewToken
" and prev == \ -> InString(span.end = pos)      (escaped quote)
_               -> InString(span.end = pos)


InNumber(span)
--------------
digit           -> InNumber(span.end = pos)
-               -> if prev not in {., -} then InNumber(span.end = pos)
                   else Invalid
e               -> if no prior 'e' and prev != '-' then InNumber(span.end = pos)
                   else Invalid
.               -> if prev not in {e, ., -} then InNumber(span.end = pos)
                   else Invalid
_               -> EMIT(Number, span.start..pos) ; NewToken
                   (do not consume current char; re-process in NewToken)


InTrue(span)
------------
prefix == "true"              -> EMIT(True, span.start..pos+1) ; NewToken
"true".starts_with(prefix)    -> InTrue(span.end = pos)
_                             -> Invalid


InFalse(span)
-------------
prefix == "false"             -> EMIT(False, span.start..pos+1) ; NewToken
"false".starts_with(prefix)   -> InFalse(span.end = pos)
_                             -> Invalid


InNull(span)
------------
prefix == "null"              -> EMIT(Null, span.start..pos+1) ; NewToken
"null".starts_with(prefix)    -> InNull(span.end = pos)
_                             -> Invalid


Invalid(message, span)
----------------------
*               -> STOP(LexicalError)
```


## Grammar
```bnf
value ::= string 
        | number
        | literal
        | array
        | object

array ::= "[" elements? "]"

object ::= "{" members? "}"

literal ::= true
          | false
          | null

pair ::= string ":" value

members ::= pair
          | pair "," members


elements ::= value 
           | value "," elements

```
