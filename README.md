### Sozluk is a command line dictionary that supports Stardict format. Heavily influenced by [sdcv](sdcv link). 

## 

* **matcher algorithms specific to a language**:  Currently none implemented. 

* **http server**: An http server running on localhost gives a chances to lookup words with the help of browser extensions. Currently there are no implementations.

* **faster search times** 

    ``` shell
    $ time sdcv -1 -0 -2 dic -n sozluk 
    real	0m0,414s
    user	0m0,188s
    sys 	0m0,045s
    
    $ time target/release/sozluk sozluk -x --paths dic
    real	0m0,124s
    user	0m0,239s
    sys 	0m0,024s 
    ```  


## Planned features: 

* dict.dz (gzip) support
* More sophisticated matching algorithms
* Interface to easily create stardict format from other dictionary formats and to automate this process

## Compiling, installing and usage:

* You need cargo to be installed on your computer.  
* `git clone ` 
* `cargo install`

## Replacing KOReader's default sdcv with sozluk: 