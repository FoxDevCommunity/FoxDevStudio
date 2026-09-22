//! The `FoxScript` namespace: how it is reached, what it says it is made of, and what it will
//! not let a program do to it.
//!
//! The specification is docs/foxscript.md, "The FoxScript namespace"; these tests are it.

use foxvm::compiler::compile_program;
use foxvm::foxscript;
use foxvm::host::HostRequest;
use foxvm::mock_host::{MockHost, run_program};
use foxvm::value::{FuncId, Handle, Value};
use foxvm::vm::{Step, Vm};

fn run(src: &str) -> Vec<String> {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn fails(src: &str) -> String {
    match run_program(src, &mut MockHost::new()) {
        Ok((_, out)) => panic!("expected an error, got {out:?}"),
        Err(e) => format!("{}: {}", e.code, e.message),
    }
}

#[test]
fn foxscript_is_an_object_and_says_so() {
    let out = run("? VARTYPE(FoxScript)\n");
    assert_eq!(out, vec!["O"]);
}

#[test]
fn a_program_with_its_own_foxscript_variable_keeps_working() {
    // the name answers only when nothing else in scope does, which is the rule that keeps
    // LAMBDA a variable name as well
    let out = run("LOCAL FoxScript\nFoxScript = \"mine\"\n? FoxScript, VARTYPE(FoxScript)\n");
    assert_eq!(out, vec!["mine C"]);
}

#[test]
fn a_private_of_that_name_wins_too_and_giving_it_up_gives_the_namespace_back() {
    let out = run(
        "? VARTYPE(FoxScript)\n\
         DO shadow\n\
         ? VARTYPE(FoxScript)\n\
         PROCEDURE shadow\n\
         PRIVATE FoxScript\n\
         FoxScript = 1\n\
         ? VARTYPE(FoxScript)\n\
         ENDPROC\n",
    );
    assert_eq!(out, vec!["O", "N", "O"]);
}

#[test]
fn amembers_lists_what_the_namespace_has() {
    let out = run(
        "LOCAL ARRAY am(1)\n\
         LOCAL n, i\n\
         n = AMEMBERS(am, FoxScript, 1)\n\
         FOR i = 1 TO n\n\
           ? am(i, 1) + \" \" + am(i, 2)\n\
         ENDFOR\n",
    );
    assert_eq!(out, vec!["DATA Object", "HTTP Object", "JSON Object", "VERSION Property"]);
}

#[test]
fn getpem_reads_a_member_the_way_it_reads_any_other() {
    let out = run("? GETPEM(FoxScript, \"Version\") == FoxScript.Version\n");
    assert_eq!(out, vec![".T."]);
}

#[test]
fn the_version_is_what_the_runtime_calls_itself() {
    let out = run("? FoxScript.Version\n");
    assert_eq!(out, vec![foxscript::VERSION.to_string()]);
}

#[test]
fn a_member_it_has_not_got_is_the_products_unknown_member() {
    assert_eq!(fails("? FoxScript.Nonesuch\n"), "1925: Unknown member NONESUCH.");
}

#[test]
fn calling_a_member_that_is_not_a_method_says_so_rather_than_reaching_the_host() {
    assert_eq!(fails("? FoxScript.Version()\n"), "1925: Unknown member VERSION.");
}

#[test]
fn a_program_may_not_write_to_the_namespace() {
    assert_eq!(fails("FoxScript.Version = \"mine\"\n"), "1743: VERSION is a read-only property.");
    assert_eq!(fails("FoxScript.Nonesuch = 1\n"), "1925: Unknown member NONESUCH.");
}

#[test]
fn the_namespaces_handles_are_out_of_the_range_the_host_allocates_from() {
    // the host counts up from 1 and keeps 0x7fff_ffff for the application object, so a native
    // handle can never be read as one of its objects - and one that escaped would be refused by
    // name rather than land on something else
    assert!(foxscript::handle(foxscript::ROOT).0 > 0x7fff_ffff);
    assert!(foxscript::in_range(foxscript::handle(foxscript::ROOT)));
    assert!(!foxscript::in_range(Handle(1)));
    assert!(!foxscript::in_range(Handle(0x7fff_ffff)));

    let natives = foxscript::Natives::default();
    assert!(natives.contains(foxscript::handle(foxscript::ROOT)));
    assert!(natives.contains(foxscript::handle(foxscript::HTTP)));
    assert!(!natives.contains(Handle(1)));
    // a handle in the range that names nothing is refused rather than read as the last object
    assert!(!natives.contains(foxscript::handle(99)));
}

// ----- the server ---------------------------------------------------------------------------

#[test]
fn createserver_makes_an_object_of_its_own() {
    let out = run(
        "LOCAL oServer\n\
         oServer = FoxScript.Http.CreateServer()\n\
         ? VARTYPE(oServer), oServer.Port\n",
    );
    assert_eq!(out, vec!["O .F."]);
}

#[test]
fn amembers_lists_what_a_server_can_do() {
    let out = run(
        "LOCAL ARRAY am(1)\n\
         LOCAL oServer, n, i, cList\n\
         oServer = FoxScript.Http.CreateServer()\n\
         n = AMEMBERS(am, oServer, 1)\n\
         cList = \"\"\n\
         FOR i = 1 TO n\n\
           cList = cList + IIF(i > 1, \" \", \"\") + am(i, 1)\n\
         ENDFOR\n\
         ? cList\n",
    );
    assert_eq!(out, vec!["CLOSE DELETE GET HEAD LISTEN OPTIONS PATCH PORT POST PUT"]);
}

#[test]
fn registering_a_route_answers_the_server_so_they_chain() {
    let out = run(
        "LOCAL oServer\n\
         oServer = FoxScript.Http.CreateServer()\n\
         ? oServer.Get(\"/a\", LAMBDA(req, res)\n\
             RETURN 1\n\
           ENDLAMBDA) = oServer\n",
    );
    assert_eq!(out, vec![".T."]);
}

#[test]
fn a_route_registered_twice_under_one_method_is_refused() {
    let src = "LOCAL oServer\n\
               oServer = FoxScript.Http.CreateServer()\n\
               oServer.Get(\"/a\", LAMBDA(req, res)\n\
                 RETURN 1\n\
               ENDLAMBDA)\n\
               oServer.Get(\"/a\", LAMBDA(req, res)\n\
                 RETURN 2\n\
               ENDLAMBDA)\n";
    assert_eq!(fails(src), "3001: GET /a already has a handler.");
}

#[test]
fn a_route_wants_a_lambda_and_says_so_when_it_gets_something_else() {
    let src = "LOCAL oServer\n\
               oServer = FoxScript.Http.CreateServer()\n\
               oServer.Get(\"/a\", \"not a lambda\")\n";
    assert_eq!(fails(src), "9: GET needs a route and a lambda to answer it with.");
}

#[test]
fn the_route_table_answers_which_handler_a_request_matches() {
    // this is what the host asks the moment a request arrives, with the VM off the stack
    let src = "PUBLIC goServer\n\
               goServer = FoxScript.Http.CreateServer()\n\
               goServer.Get(\"/api/v1/customers/:id\", LAMBDA(req, res)\n\
                 RETURN \"customer\"\n\
               ENDLAMBDA)\n\
               goServer.Post(\"/api/v1/customers\", LAMBDA(req, res)\n\
                 RETURN \"new customer\"\n\
               ENDLAMBDA)\n";
    let (vm, _) = run_program(src, &mut MockHost::new()).expect("runs");
    let Some(Value::Object(server)) = vm.get_global("GOSERVER") else { panic!("no server") };

    let (func, params) = vm.route(server, "GET", "/api/v1/customers/42").expect("a route");
    assert_eq!(params, vec![("id".to_string(), "42".to_string())]);
    assert!(vm.function_value(FuncId(func)).is_some(), "the id names a function the VM still has");

    assert!(vm.route(server, "GET", "/api/v1/customers").is_none(), "the collection is not the item");
    assert!(vm.route(server, "POST", "/api/v1/customers/42").is_none(), "a method has its own routes");
    assert!(vm.route(server, "POST", "/api/v1/customers").is_some());
    // the match is case-insensitive in the method and exact in the path, as HTTP is
    assert!(vm.route(server, "get", "/api/v1/customers/42").is_some());
}

#[test]
fn listen_asks_the_host_for_a_socket_and_keeps_the_port_it_got() {
    let src = "PUBLIC gnPort, goServer\n\
               goServer = FoxScript.Http.CreateServer()\n\
               gnPort = goServer.Listen(8080)\n";
    let mut host = MockHost::new();
    let mut vm = Vm::new();
    let module = compile_program(src, "main.prg").module.expect("compiles");
    let id = vm.load_module(module);
    let fiber = vm.start(id, 0, None, Vec::new());
    let mut asked = Vec::new();
    loop {
        match vm.step(&mut host, fiber) {
            Step::Done { .. } => break,
            Step::Error(e) => panic!("{}: {}", e.code, e.message),
            Step::Suspend(req) => {
                asked.push(req);
                // the host bound a different port from the one that was asked for, which is
                // what happens when a program asks for 0
                vm.resume(fiber, Value::number(9090.0));
            }
        }
    }
    let Some(Value::Object(server)) = vm.get_global("GOSERVER") else { panic!("no server") };
    assert_eq!(asked, vec![HostRequest::HttpListen { server: server.0, port: 8080 }]);
    assert_eq!(vm.get_global("GNPORT"), Some(Value::number(9090.0)));
}
