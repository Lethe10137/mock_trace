cargo run --release

# rm mock_trace.zip
# rm mock_toml.zip

zip -r mock_trace mock_trace
zip -r mock_toml mock_toml

# rsync -avz mock_trace.zip lich@101.6.30.160:~/upload/
# rsync -avz mock_toml.zip lich@101.6.30.160:~/upload/