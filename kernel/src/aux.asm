bits 64

; GONE! This was 32 bit only
;
; global panic_unwind_capture_state
;
; This function is a little tricky. For the unwinder to work we need to capture
; the state of the call stack as it exists. Returning ESP and RA would mess up
; the top of the stack so instead we need to pass this data to a callback.
;
; The callback also needs an opaque data pointer so that we can access context
; in this callback without passing the context through globals.
;
; extern "C" {
;     fn panic_unwind_capture_state(
;         data: *mut c_void,
;         f: extern fn(data: *mut c_void, reg: *const CReg),
;     );
; }
; panic_unwind_capture_state:
;     ; set up frame pointer
;     push ebp
;     mov ebp, esp

;     ; load + push return address
;     mov eax, [ebp + 4]
;     push eax

;     ; load + push what stack pointer would have been
;     lea eax, [ebp + 8]
;     push eax

;     ; push pointer to CReg on stack
;     push esp

;     ; push cb arg
;     mov eax, [ebp + 8]
;     push eax

;     ; load func arg
;     mov eax, [ebp + 12]
;     call eax

;     ; tear down frame pointer and return
;     mov esp, ebp
;     pop ebp
;     ret
