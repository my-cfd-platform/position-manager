fn main() {
    let url = "https://raw.githubusercontent.com/my-cfd-platform/proto-files/main/proto/";
    ci_utils::sync_and_build_proto_file(url, "PositionsManager.proto");
    ci_utils::sync_and_build_proto_file(url, "PositionsManagerPersistence.proto");
}
