namespace MyCompany.Core

type PatientId = PatientId of string

type Patient = {
    Id: PatientId
    Name: string
    Age: int
}

type IValidator =
    abstract member Validate: Patient -> bool

type BaseValidator() =
    member this.Version = "1.0.0"

type PatientValidator() =
    inherit BaseValidator()
    interface IValidator with
        member this.Validate(patient: Patient) =
            patient.Age >= 0 && not (System.String.IsNullOrWhiteSpace(patient.Name))

module PatientValidation =
    let (|ValidAge|InvalidAge|) age =
        if age >= 0 then ValidAge else InvalidAge

    let validatePatient (patient: Patient) =
        match patient.Age with
        | ValidAge -> patient.Name <> ""
        | InvalidAge -> false
