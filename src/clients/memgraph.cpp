#include "memgraph.h"

#include <cassert>
#include <iostream>
#include <string>

static void begin_transaction(MgClient &client) {
  // assert(client.BeginTransaction());
  assert(client.Execute("BEGIN"));
  client.DiscardAll();
}

static void commit_transaction(MgClient &client) {
  // assert(client.CommitTransaction());
  assert(client.Execute("COMMIT"));
  client.DiscardAll();
}

void init() { mg::Client::Init(); }

std::unique_ptr<MgClient> new_client(rust::Str ip, uint16_t port) {
  mg::Client::Params params;
  params.host = std::string(ip);
  params.port = port;
  return mg::Client::Connect(params);
}

void exec_transaction(MgClient &client, rust::Vec<Event> &transaction) {
  bool success = false;
  std::string write_stmt = "MATCH (n:KV {var: $var}) SET n.val = $val RETURN n.val;",
              read_stmt = "MATCH (n:KV {var: $var}) RETURN n.val;";

  while (!success) {
    begin_transaction(client);
    success = true;

    try {
      for (auto &ev : transaction) {
        mg::Map map{{"var", mg::Value(ev.key)}, {"val", mg::Value(ev.value)}};

        if (ev.event_type == EventType::Read) {
          if (!client.Execute(read_stmt, map.AsConstMap())) {
            success = false;
            break;
          }

          auto result = client.FetchAll().value();
          // std::cerr << "result: " << result.size() << result[0].size() << result[0][0].ValueInt() << '\n';
          ev.value = result[0][0].ValueInt();
        } else {
          if (!client.Execute(write_stmt, map.AsConstMap())) {
            success = false;
            break;
          }

          auto result = client.FetchAll().value();
          assert(ev.value == result[0][0].ValueInt());
        }
      }
    } catch (mg::ClientException &e) {
      success = false;
    }

    if (success) {
      commit_transaction(client);
    }
  }
}

void create_variables(MgClient &client, int64_t n_variables) {
  std::string create_stmt = "CREATE (n:KV {var: $var, val: $val});";
  begin_transaction(client);
  for (int64_t i = 0; i < n_variables; i++) {
    mg::Map map{{"var", mg::Value(i)}, {"val", mg::Value(0)}};
    assert(client.Execute(create_stmt, map.AsConstMap()));
    client.DiscardAll();
  }
  commit_transaction(client);
}

void drop_database(MgClient &client) {
  assert(client.Execute("MATCH (n:KV) DELETE n;"));
  client.DiscardAll();
}
