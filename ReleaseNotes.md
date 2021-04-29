# Release 0.17.0
Implemented features:
- Implemented throttling in case of invalid QES
- Implemented middleware to log incomming requests and corresponding responses
- Implemented RevInclude for AuditEvents 

Bugfixes / Improvements:
- Check siging time while verifying QES signature
- Store communication count of each task on disc
- Fixed CAdES signature (this fix needs openssl 3.0.0, see README for more details)
- Fixed panic in thask close operation
- Fixed FHIR profiles

Developer Hints:
- This release will break the format of the state stored to disc!


# Release 0.16.0
Implemented features:
- Auto-Delete timed out Resources

Bugfixes / Improvements:
- Communiation Delete should be restricted to "sender"
- Medication as contained Resource in MedicationDispense
- Limit Text Size of XML / JSON parser to 1MB
- Missing User-Agent Header should return 403


# Release 0.15.1
Implemented features:
- Added counter for Communication resource
- Added Healthcheck Endpoint

Hotfixes:
- /Task/$accept is missing Task.secret


# Release 0.15.0
Implemented features:
- Sign Receipt Bundle
- Added /VAUCertificateOCSPResponse endpoint
- Added /OCSPList endpoint with list of OCSP respponses
- Added /Random endpoint

Developer hints:
- The place of the certificate that is stored in the signature of the receipt bundle may be changed in the future


# Release 0.14.0
Implemented features:
- Added crates to handle XAdES
- Added attachment to Communication.content
- Fixed profile of AuditEvent
- Added search parameters to CapabilityStatement
- Fixed XML encoding
Developer hints:
- This version will break the format of the state stored to disc

# Release 0.13.0
Implemented features:
* FD server endpoint providing certifcate list
* Different Bug Fixes

# Release 0.12.0
Implemented features:
* Implement Task GET for Supplier Interface
* Audit-Events creation
* Unique Prescription ID
* Persist State to File

# Release 0.11.0
Implemented features:
* Implement Task History
* Validate IDP Public Key against CA-Certificate in TSL
* Updated rust openssl to v0.10.32

# Release 0.10.0
Implemented features:
* Fixed CMS signature verification
* Updated KBV FHIR Profile
* Fixed JSON signature calculation

# Release 0.9.0
Implemented features:
* Implemented KBV eRezept FHIR Profile update
* Implemented expiryDate and acceptDate for Task $activate operation

# Release 0.8.0
Implemented features:
* Implemented OperationOutcome Resource for Errorneous Operations
* Implemented AuditEvent Resource and Routes
* Implemented Device Resource
* Improved Resource Filtering
* Implemented Task $accept Operation
* Implemented Task $reject Operation
* Implemented Task $close Operation

# Release 0.7.0
Implemented features:
* Implemented FD signature for KBV bundle
* Load CA Certificats form BNetz-A VL to verify QES container
* Added version information (from git repository) to capability statement
* Added simple script to generate test data

# Release 0.6.0
Implemented features:
* Load PUK_TOKEN certificate from IDP
* Verify ACCESS_TOKEN of VAU protocol
* Added tool to support developers
* Implemented support for NO_PROXY environment variable
* Implemented Task $abort operation
* Implemented MedicationDispense resource
* Implemented paging, sorting and filtering for Task resources
* Refactored XML/JSON parsing to support streaming

# Release 0.5.0
Implemented features:
* Fetch public key from IDP to verify ACCESS_TOKEN
* Fixed cipher algorithm used for ACCESS_TOKEN verification

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

