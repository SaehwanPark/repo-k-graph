module Library.Operations

let helper () =
    printfn "Helper called"

let run () =
    helper ()

type User = { Name: string }
