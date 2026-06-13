// F# Script Example
open MyCompany.Core

let testPatient = {
    Id = PatientId "123"
    Name = "Alice"
    Age = 30
}

let isValid = PatientValidation.validatePatient testPatient
printfn "Patient validity: %b" isValid
