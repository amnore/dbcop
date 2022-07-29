#pragma once

#include "rust/cxx.h"
#include "mgclient.hpp"

#include <cstddef>
#include <cstdint>
#include <memory>

using MgClient = mg::Client;

#include "dbcop/src/clients/memgraph.rs.h"

void init();
std::unique_ptr<MgClient> new_client(rust::Str ip, uint16_t port);
void exec_transaction(MgClient &client, rust::Vec<Event> &transaction);
void create_variables(MgClient &client, int64_t n_variables);
void drop_database(MgClient &client);
