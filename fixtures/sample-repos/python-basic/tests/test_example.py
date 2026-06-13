from src.example import validate_patient


def test_validate_patient_accepts_id():
  assert validate_patient({"id": "patient-1"})
