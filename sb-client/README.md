Silverbullet command line client 

# przykłady
## wywołanie funkcji
    `sb-client <function-name>`
    działa, ale po zmianie funkcji na serwerze trzeba go zrestartować żeby zobaczyć zmianę przez API
    
## skrypt
    `sb-client -s -e 'return 1 + 1'`

## wyrażenie
    `sb-client -e 'query[[from b = index.tag("space-lua") where b.page == "my-functions/tasks" select b.ref]]'`

## przesłanie lokalnego skrkyptu i uruchomienie na serwerze, return ze skryptu jest zwracaną odpowiedzią
    `sb-client -f lua/next-task.lua ` - przesyła skrypt lua i uruchamia na serwerze - to działa ok

## pobranie strony    
    `sb-client space.readPage my-functions/tasks`
