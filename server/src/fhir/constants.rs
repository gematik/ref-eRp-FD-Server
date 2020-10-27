/*
 * Copyright (c) 2020 gematik GmbH
 * 
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * 
 *    http://www.apache.org/licenses/LICENSE-2.0
 * 
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

/* Parameter Types */
pub const PARAMETER_TYPE_TASK_CREATE: &str = "workflowType";
pub const PARAMETER_TYPE_TASK_ACTIVATE: &str = "ePrescription";

/* Binary Content Types */
pub const BINARY_CONTENT_TYPE_PKCS7: &str = "application/pkcs7-mime";

/* Coding Systems */
pub const CODING_SYSTEM_FLOW_TYPE: &str = "https://gematik.de/fhir/CodeSystem/Flowtype";
pub const CODING_SYSTEM_DOCUMENT_TYPE: &str = "https://gematik.de/fhir/CodeSystem/Documenttype";
pub const CODING_SYSTEM_PERFORMER_TYPE: &str = "urn:ietf:rfc:3986";
pub const CODING_SYSTEM_COMPOSITION: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Formular_Art";
pub const CODING_SYSTEM_LEGAL_BASIS: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_SFHIR_KBV_STATUSKENNZEICHEN";
pub const CODING_SYSTEM_SECTION: &str = "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Section_Type";
pub const CODING_SYSTEM_CO_PAYMENT: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_ERP_StatusCoPayment";
pub const CODING_SYSTEM_ACCIDENT_CAUSE: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_FOR_Ursache_Type";
pub const CODING_SYSTEM_MEDICATION_TYPE: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_ERP_Medication_Type";
pub const CODING_SYSTEM_PZN: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_ERP_PZN";
pub const CODING_SYSTEM_MEDICATION_CATEGORY: &str =
    "https://fhir.kbv.de/CodeSystem/KBV_CS_ERP_Medication_Category";
pub const CODING_SYSTEM_ASK: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_ERP_ASK";
pub const CODING_SYSTEM_IDENTIFIER_BASE: &str =
    "http://fhir.de/CodeSystem/identifier-type-de-basis";
pub const CODING_SYSTEM_V2_0203: &str = "http://terminology.hl7.org/CodeSystem/v2-0203";
pub const CODING_SYSTEM_AVAILABILITY_STATUS: &str =
    "https://gematik.de/fhir/CodeSystem/AvailabilityStatus";

/* Contact Point Systems */
pub const CONTACT_POINT_SYSTEM_PHONE: &str = "phone";
pub const CONTACT_POINT_SYSTEM_FAX: &str = "fax";
pub const CONTACT_POINT_SYSTEM_EMAIL: &str = "email";

/* Quantity System */
pub const QUANTITY_SYSTEM_MEDICATION: &str = "http://unitsofmeasure.org";

/* Identity Systems */
pub const IDENTITY_SYSTEM_KVID: &str = "http://fhir.de/NamingSystem/gkv/kvid-10";
pub const IDENTITY_SYSTEM_KVK: &str = "http://fhir.de/NamingSystem/gkv/kvk-versichertennummer";
pub const IDENTITY_SYSTEM_ANR: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_Base_ANR";
pub const IDENTITY_SYSTEM_ZANR: &str = "http://fhir.de/NamingSystem/kzbv/zahnarztnummer";
pub const IDENTITY_SYSTEM_IKNR: &str = "http://fhir.de/NamingSystem/arge-ik/iknr";
pub const IDENTITY_SYSTEM_BSNR: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_Base_BSNR";
pub const IDENTITY_SYSTEM_TEAM_NUMBER: &str = "http://fhir.de/NamingSystem/asv/teamnummer";
pub const IDENTITY_SYSTEM_TELEMATIK_ID: &str = "https://gematik.de/fhir/Namingsystem/TelematikID";

/* Identifier Systems */
pub const IDENTIFIER_SYSTEM_PRESCRIPTION_ID: &str =
    "https://gematik.de/fhir/Namingsystem/PrescriptionID";
pub const IDENTIFIER_SYSTEM_ACCESS_CODE: &str = "https://gematik.de/fhir/Namingsystem/AccessCode";
pub const IDENTIFIER_SYSTEM_SECRET: &str = "https://gematik.de/fhir/Namingsystem/Secret";
pub const IDENTIFIER_SYSTEM_PRF: &str = "https://fhir.kbv.de/NamingSystem/KBV_NS_FOR_Pruefnummer";

/* Resource Profiles */
pub const RESOURCE_PROFILE_TASK: &str = "https://gematik.de/fhir/StructureDefinition/erxTask";
pub const RESOURCE_PROFILE_COMPOSITION: &str =
    "https://gematik.de/fhir/StructureDefinition/erxComposition";
pub const RESOURCE_PROFILE_KBV_BUNDLE: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Bundle|1.00.000";
pub const RESOURCE_PROFILE_MEDICATION_REQUEST: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Prescription|1.00.000";
pub const RESOURCE_PROFILE_MEDICATION_COMPOUNDING: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_Compounding|1.00.000";
pub const RESOURCE_PROFILE_MEDICATION_FREE_TEXT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_FreeText|1.00.000";
pub const RESOURCE_PROFILE_MEDICATION_INGREDIENT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_Ingredient|1.00.000";
pub const RESOURCE_PROFILE_MEDICATION_PZN: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_ERP_Medication_PZN|1.00.000";
pub const RESOURCE_PROFILE_PATIENT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Patient|1.0.1";
pub const RESOURCE_PROFILE_PRACTITIONER: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Practitioner|1.0.1";
pub const RESOURCE_PROFILE_ORGANIZATION: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Organization|1.0.1";
pub const RESOURCE_PROFILE_COVERAGE: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_Coverage|1.0.1";
pub const RESOURCE_PROFILE_PRACTITIONER_ROLE: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_PR_FOR_PractitionerRole|1.0.1";
pub const RESOURCE_PROFILE_COMMUNICATION: &str =
    "http://hl7.org/fhir/StructureDefinition/Communication";
pub const RESOURCE_PROFILE_COMMUNICATION_INFO_REQ: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationInfoReq";
pub const RESOURCE_PROFILE_COMMUNICATION_REPLY: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationReply";
pub const RESOURCE_PROFILE_COMMUNICATION_DISPENSE_REQ: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationDispReq";
pub const RESOURCE_PROFILE_COMMUNICATION_REPRESENTATIVE: &str =
    "https://gematik.de/fhir/StructureDefinition/erxCommunicationRepresentative";

/* Extension Urls */
pub const EXTENSION_URL_PRESCRIPTION: &str =
    "https://gematik.de/fhir/StructureDefinition/PrescriptionType";
pub const EXTENSION_URL_ACCEPT_DATE: &str =
    "https://example.org/fhir/StructureDefinition/AcceptDate";
pub const EXTENSION_URL_EXPIRY_DATE: &str =
    "https://gematik.de/fhir/StructureDefinition/ExpiryDate";
pub const EXTENSION_URL_LEGAL_BASIS: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_FOR_Legal_basis";
pub const EXTENSION_URL_EMERGENCY_SERVICE_FEE: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_EmergencyServicesFee";
pub const EXTENSION_URL_BGV: &str = "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_BVG";
pub const EXTENSION_URL_DOSAGE_FLAG: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_DosageFlag";
pub const EXTENSION_URL_CO_PAYMENT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_StatusCoPayment";
pub const EXTENSION_URL_ACCIDENT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Accident";
pub const EXTENSION_URL_ACCIDENT_CAUSE: &str = "unfallkennzeichen";
pub const EXTENSION_URL_ACCIDENT_DATE: &str = "unfalltag";
pub const EXTENSION_URL_ACCIDENT_BUSINESS: &str = "unfallbetrieb";
pub const EXTENSION_URL_MEDICATION_CATEGORY: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Category";
pub const EXTENSION_URL_MEDICATION_VACCINE: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Vaccine";
pub const EXTENSION_URL_MEDICATION_INSTRUCTION: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_compoundingInstruction";
pub const EXTENSION_URL_MEDICATION_PACKAGING: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Packaging";
pub const EXTENSION_URL_STANDARD_SIZE: &str = "http://fhir.de/StructureDefinition/normgroesse";
pub const EXTENSION_URL_MEDICATION_INGREDIENT_FORM: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Ingredient_Form";
pub const EXTENSION_URL_MEDICATION_INGREDIENT_AMOUNT: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_ERP_Medication_Ingredient_Amount";
pub const EXTENSION_URL_HUMAN_NAME_OWN_NAME: &str =
    "http://hl7.org/fhir/StructureDefinition/humanname-own-name";
pub const EXTENSION_URL_HUMAN_NAME_EXTENSION: &str =
    "http://fhir.de/StructureDefinition/humanname-namenszusatz";
pub const EXTENSION_URL_HUMAN_NAME_PREFIX: &str =
    "http://hl7.org/fhir/StructureDefinition/humanname-own-prefix";
pub const EXTENSION_URL_ADDRESS_STREET: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-streetName";
pub const EXTENSION_URL_ADDRESS_NUMBER: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-houseNumber";
pub const EXTENSION_URL_ADDRESS_ADDITION: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-additionalLocator";
pub const EXTENSION_URL_ADDRESS_POST_BOX: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-ADXP-postBox";
pub const EXTENSION_URL_ISO21090_EN: &str =
    "http://hl7.org/fhir/StructureDefinition/iso21090-EN-qualifier";
pub const EXTENSION_URL_SPECIAL_GROUP: &str =
    "http://fhir.de/StructureDefinition/gkv/besondere-personengruppe";
pub const EXTENSION_URL_DMP_MARK: &str = "http://fhir.de/StructureDefinition/gkv/dmp-kennzeichen";
pub const EXTENSION_URL_INSURED_TYPE: &str =
    "http://fhir.de/StructureDefinition/gkv/versichertenart";
pub const EXTENSION_URL_WOP: &str = "http://fhir.de/StructureDefinition/gkv/wop";
pub const EXTENSION_URL_ALTERNATIVE_IK: &str =
    "https://fhir.kbv.de/StructureDefinition/KBV_EX_FOR_Alternative_IK";
pub const EXTENSION_URL_INSURANCE_PROVIDER: &str =
    "https://gematik.de/fhir/StructureDefinition/InsuranceProvider";
pub const EXTENSION_URL_SUPPLY_OPTIONS: &str =
    "https://gematik.de/fhir/StructureDefinition/SupplyOptionsType";
pub const EXTENSION_URL_SUBSTITUTION_ALLOWED: &str =
    "https://gematik.de/fhir/StructureDefinition/SubstitutionAllowedType";
pub const EXTENSION_URL_AVAILABILITY_STATUS: &str =
    "https://gematik.de/fhir/StructureDefinition/AvailabilityStatus";

/* Resource Types */
pub const RESOURCE_TYPE_TASK: &str = "Task";
pub const RESOURCE_TYPE_PARAMETERS: &str = "Parameters";
pub const RESOURCE_TYPE_COMPOSITION: &str = "Composition";
pub const RESOURCE_TYPE_KBV_BUNDLE: &str = "Bundle";
pub const RESOURCE_TYPE_MEDICATION_REQUEST: &str = "MedicationRequest";
pub const RESOURCE_TYPE_MEDICATION: &str = "Medication";
pub const RESOURCE_TYPE_PATIENT: &str = "Patient";
pub const RESOURCE_TYPE_PRACTITIONER: &str = "Practitioner";
pub const RESOURCE_TYPE_ORGANIZATION: &str = "Organization";
pub const RESOURCE_TYPE_COVERAGE: &str = "Coverage";
pub const RESOURCE_TYPE_PRACTITIONER_ROLE: &str = "PractitionerRole";
pub const RESOURCE_TYPE_BUNDLE: &str = "Bundle";
pub const RESOURCE_CAPABILITY_STATEMENT: &str = "CapabilityStatement";
pub const RESOURCE_TYPE_COMMUNICATION: &str = "Communication";

/* XMLNS */
pub const XMLNS_TASK: &str = "http://hl7.org/fhir";
pub const XMLNS_PARAMETERS: &str = "http://hl7.org/fhir";
pub const XMLNS_COMPOSITION: &str = "http://hl7.org/fhir";
pub const XMLNS_KBV_BUNDLE: &str = "http://hl7.org/fhir";
pub const XMLNS_MEDICATION_REQUEST: &str = "http://hl7.org/fhir";
pub const XMLNS_MEDICATION: &str = "http://hl7.org/fhir";
pub const XMLNS_PATIENT: &str = "http://hl7.org/fhir";
pub const XMLNS_PRACTITIONER: &str = "http://hl7.org/fhir";
pub const XMLNS_ORGANIZATION: &str = "http://hl7.org/fhir";
pub const XMLNS_COVERAGE: &str = "http://hl7.org/fhir";
pub const XMLNS_PRACTITIONER_ROLE: &str = "http://hl7.org/fhir";
pub const XMLNS_BUNDLE: &str = "http://hl7.org/fhir";
pub const XMLNS_CAPABILITY_STATEMENT: &str = "http://hl7.org/fhir";
pub const XMLNS_COMMUNICATION: &str = "http://hl7.org/fhir";

/* Constant Values */
pub const TASK_INTENT: &str = "order";

/* Composition */
pub const COMPOSITION_STATUS: &str = "final";
pub const COMPOSITION_TYPE_CODE: &str = "e16A";
pub const COMPOSITION_ATTESTER_MODE: &str = "legal";
pub const COMPOSITION_TYPE_AUTHOR_DOCTOR: &str = "Practitioner";
pub const COMPOSITION_TYPE_AUTHOR_PRF: &str = "Device";
pub const COMPOSITION_CODE_SECTION_REGULATION: &str = "Verordnung";
pub const COMPOSITION_CODE_SECTION_COVERAGE: &str = "Coverage";
pub const COMPOSITION_CODE_SECTION_PRACTITIONER_ROLE: &str = "FOR_PractitionerRole";

/* Medication Request */
pub const MEDICATION_REQUEST_STATUS: &str = "active";
pub const MEDICATION_REQUEST_INTENT: &str = "order";
pub const MEDICATION_REQUEST_QUANTITY_CODE: &str = "{PACK}";

/* Medication */
pub const MEDICATION_TYPE_CODE_COMPOUNDING: &str = "rezeptur";
pub const MEDICATION_TYPE_CODE_FREE_TEXT: &str = "freitext";
pub const MEDICATION_TYPE_CODE_INGREDIENT: &str = "wirkstoff";

/* Patient */
pub const PATIENT_IDENTIFIER_GKV: &str = "GKV";
pub const PATIENT_IDENTIFIER_PKV: &str = "PKV";
pub const PATIENT_IDENTIFIER_KVK: &str = "kvk";

/* Pracitioner */
pub const PRACTITIONER_CODE_LANR: &str = "LANR";
pub const PRACTITIONER_CODE_ZANR: &str = "ZANR";

/* Organization */
pub const ORGANIZATION_IDENTIFIER_CODE_IKNR: &str = "XX";
pub const ORGANIZATION_IDENTIFIER_CODE_BSNR: &str = "BSNR";
pub const ORGANIZATION_IDENTIFIER_CODE_ZANR: &str = "ZANR";

/* Operations */
pub const OPERATION_TASK_CREATE: &str =
    "http://gematik.de/fhir/OperationDefinition/CreateOperationDefinition";
pub const OPERATION_TASK_ACCEPT: &str =
    "http://gematik.de/fhir/OperationDefinition/AcceptOperationDefinition";
pub const OPERATION_TASK_ACTIVATE: &str =
    "http://gematik.de/fhir/OperationDefinition/ActivateOperationDefinition";
pub const OPERATION_TASK_ABORT: &str =
    "http://gematik.de/fhir/OperationDefinition/AbortOperationDefinition";
pub const OPERATION_TASK_CLOSE: &str =
    "http://gematik.de/fhir/OperationDefinition/CloseOperationDefinition";
pub const OPERATION_TASK_REJECT: &str =
    "http://gematik.de/fhir/OperationDefinition/RejectOperationDefinition";
