# secure_openaiapi by AIvantGuard
Secure OpenAI API calls - ZeroTrust repository


## About the project

We created this repository, to demo how OpenAI compatible APIs can be called in a secure way.
We know most of the currently used implementation insecure on the client side.

With this demo library show the ZeroTrust mindset which we use in our solutions.

For more details about the problem:

[AIvantGuard's LinkedIn Post](https://www.linkedin.com/posts/aioneguard_%F0%9D%97%98%F0%9D%97%A1%F0%9D%97%9A%F0%9D%97%9C%F0%9D%97%A1%F0%9D%97%98%F0%9D%97%98%F0%9D%97%A5%F0%9D%97%9C%F0%9D%97%A1%F0%9D%97%9A-%F0%9D%97%A8%F0%9D%97%A3%F0%9D%97%97%F0%9D%97%94%F0%9D%97%A7%F0%9D%97%98-%F0%9D%9F%AE%F0%9D%9F%AC-activity-7342424215205101568-MgpW)

## Building the project

before building it requires cargo and rust

```shell

python -m venv .venv
source activate .venv/bin/activate
pip install --upgrade pip
pip install maturin
```

Building the library
```shell

maturin build --release
```

Building for development
```shell

maturin develop
```

## Running the example
```shell

python secure_usage_example.py
```

For usage check the code of the [secure_usage_example.py](secure_usage_example.py)