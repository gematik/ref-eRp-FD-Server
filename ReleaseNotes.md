# Release 0.4.0
Implemented features:
* Verify KBV Bundle Signature on Task activate
* Process Certificates of TSL
* Return all referenced resources on Task GET

# Release 0.3.0
Implemented features:
* Download and provide endpoints for TSL (Trust Status List)
* Updated actix-web to 3.0
* Extended Readme

# Release 0.2.0
Implemented features:
* FHIR resources and operations
  * Communication resource
    * create interaction
    * read interaction
    * delete interaction
* Access token validation

# Release 0.1.0
Implemented features:
* REST server basics
* eRp VAU protocol encryption and decryption
* FHIR server basics:
  * Capability Statement generation
  * XML and JSON serialization and deserialization
  * _format parameter handling for Capability Statement
* FHIR resources and operations
  * Task resource
    * read interaction
    * $create operation
    * $activate operation
* access code generation
* separate interfaces for eRp-App (FdV) and medical suppliers/pharmacies (LE)

