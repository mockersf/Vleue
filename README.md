#  Vleue [![Build Status](https://travis-ci.org/mockersf/Vleue.svg?branch=master)](https://travis-ci.org/mockersf/Vleue)

ToDo api built using rust and deployed on AWS Lambda


## Dependencies
* [rust-crowbar](https://github.com/ilianaw/rust-crowbar) for python wrapper to AWS Lambda
* [rusoto](https://github.com/rusoto/rusoto) as the AWS SDK (access to dynamoDB for now)
* [failure](https://github.com/withoutboats/failure) for error management
* [frank_jwt](https://github.com/GildedHonour/frank_jwt) for JWE tokens
* [serde](https://github.com/serde-rs/serde) for JSON serialization


## Deployment

Can be deployed using [Serverless](https://serverless.com). An environment variable specifying the DynamoDB table name must be provided.

RSA Keys must be present in folder "keys". To generate them:
```
ssh-keygen -t rsa -b 4096 -f jwtRS256.key
openssl rsa -in jwtRS256.key -pubout -outform PEM -out jwtRS256.pem.key.pub
```
